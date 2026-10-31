mod limits;
use limits::*;

mod fetcher;
use fetcher::*;

use std::{
    collections::HashMap,
    io::{self, Read, Seek, SeekFrom},
    sync::{Arc, LazyLock, Mutex, Weak},
    time::{Duration, Instant},
};
use symphonia::core::io::MediaSource;
mod cache;
use cache::Cache;
use std::sync::atomic::{AtomicBool, Ordering};
const PAGE: u64 = 256 * 1024;
const MAX_WINDOW: u64 = 4 * 1024 * 1024;
#[derive(Clone)]
pub struct StreamOptions {
    pub cache_dir: Option<std::path::PathBuf>,
    pub cache_key: String,
    pub cache_bytes: u64,
    pub prefetch: bool,
}
impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            cache_dir: None,
            cache_key: String::new(),
            cache_bytes: 2 * 1024 * 1024 * 1024,
            prefetch: true,
        }
    }
}

type Resolver = dyn Fn() -> Result<String, String> + Send + Sync;
static SOURCES: LazyLock<Mutex<HashMap<String, Weak<StreamRegistration>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct StreamRegistration {
    path: String,
    size: u64,
    pub(crate) duration_ms: u64,
    pub(crate) extension: String,
    resolve: Box<Resolver>,
    options: StreamOptions,
}
impl StreamRegistration {
    pub fn new(
        size: u64,
        duration_ms: u64,
        extension: String,
        resolve: impl Fn() -> Result<String, String> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Self::with_options(
            size,
            duration_ms,
            extension,
            StreamOptions::default(),
            resolve,
        )
    }
    pub fn with_options(
        size: u64,
        duration_ms: u64,
        extension: String,
        options: StreamOptions,
        resolve: impl Fn() -> Result<String, String> + Send + Sync + 'static,
    ) -> Arc<Self> {
        let source = Arc::new(Self {
            path: format!("cloud-stream://{}", uuid::Uuid::new_v4()),
            size,
            duration_ms,
            extension,
            resolve: Box::new(resolve),
            options,
        });
        SOURCES
            .lock()
            .unwrap()
            .insert(source.path.clone(), Arc::downgrade(&source));
        source
    }
    pub fn path(&self) -> &str {
        &self.path
    }
}
impl Drop for StreamRegistration {
    fn drop(&mut self) {
        if let Ok(mut sources) = SOURCES.lock() {
            sources.remove(&self.path);
        }
    }
}
pub(crate) fn lookup(path: &str) -> Result<Arc<StreamRegistration>, String> {
    SOURCES
        .lock()
        .map_err(|_| "Cloud stream registry unavailable")?
        .get(path)
        .and_then(Weak::upgrade)
        .ok_or_else(|| "Cloud stream expired. Play the track again.".into())
}

pub fn clear_playback_cache(root: &std::path::Path) -> io::Result<()> {
    cache::clear(root)
}

struct Window {
    start: u64,
    bytes: Vec<u8>,
}
type Demand = (u64, crossbeam_channel::Sender<Result<Window, String>>);
pub(crate) struct HttpSource {
    size: u64,
    position: u64,
    window: Window,
    request: crossbeam_channel::Sender<Demand>,
    stopping: Arc<AtomicBool>,
}
impl HttpSource {
    pub(crate) fn new(source: Arc<StreamRegistration>) -> Result<Self, String> {
        let (request, demands) = crossbeam_channel::unbounded::<Demand>();
        let size = source.size;
        let stopping = Arc::new(AtomicBool::new(false));
        let stopped = stopping.clone();
        std::thread::Builder::new()
            .name("cloud-audio-buffer".into())
            .spawn(move || {
                let mut fetcher = match Fetcher::new(source.clone(), stopped.clone()) {
                    Ok(fetcher) => fetcher,
                    Err(error) => {
                        if let Ok((_, reply)) = demands.recv() {
                            let _ = reply.send(Err(error));
                        }
                        return;
                    }
                };
                let mut ahead: Option<Window> = None;
                while let Ok((position, reply)) = demands.recv() {
                    if stopped.load(Ordering::Relaxed) {
                        break;
                    }
                    let result = if ahead.as_ref().is_some_and(|window| {
                        position >= window.start
                            && position < window.start + window.bytes.len() as u64
                    }) {
                        Ok(ahead.take().unwrap())
                    } else {
                        fetcher.position = position;
                        fetcher
                            .fill()
                            .map(|_| Window {
                                start: fetcher.start,
                                bytes: std::mem::take(&mut fetcher.buffer),
                            })
                            .map_err(|e| e.to_string())
                    };
                    let next = result
                        .as_ref()
                        .ok()
                        .map(|window| window.start + window.bytes.len() as u64);
                    if reply.send(result).is_err() {
                        break;
                    }
                    if source.options.prefetch
                        && !stopped.load(Ordering::Relaxed)
                        && demands.is_empty()
                    {
                        if let Some(next) = next.filter(|next| *next < size) {
                            fetcher.position = next;
                            if fetcher.fill().is_ok() {
                                ahead = Some(Window {
                                    start: fetcher.start,
                                    bytes: std::mem::take(&mut fetcher.buffer),
                                });
                            }
                        }
                    }
                }
            })
            .map_err(|_| "Could not start audio buffering")?;
        Ok(Self {
            size,
            position: 0,
            window: Window {
                start: 0,
                bytes: Vec::new(),
            },
            request,
            stopping,
        })
    }
}
impl Drop for HttpSource {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Relaxed);
    }
}
impl Read for HttpSource {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() || self.position >= self.size {
            return Ok(0);
        }
        if self.position < self.window.start
            || self.position >= self.window.start + self.window.bytes.len() as u64
        {
            let (reply, result) = crossbeam_channel::bounded(1);
            self.request
                .send((self.position, reply))
                .map_err(|_| io::Error::other("Audio buffering stopped"))?;
            self.window = result
                .recv()
                .map_err(|_| io::Error::other("Audio buffering stopped"))?
                .map_err(io::Error::other)?;
        }
        let start = (self.position - self.window.start) as usize;
        let length = out.len().min(self.window.bytes.len() - start);
        out[..length].copy_from_slice(&self.window.bytes[start..start + length]);
        self.position += length as u64;
        Ok(length)
    }
}
impl Seek for HttpSource {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let target = match from {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::Current(n) => self.position as i128 + n as i128,
            SeekFrom::End(n) => self.size as i128 + n as i128,
        };
        if !(0..=u64::MAX as i128).contains(&target) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid stream seek",
            ));
        }
        self.position = target as u64;
        Ok(self.position)
    }
}
impl MediaSource for HttpSource {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.size)
    }
}

pub enum LibrarySource {
    Local(std::path::PathBuf),
    Stream(Arc<StreamRegistration>),
}
type LibraryResolver = dyn Fn(&str) -> Result<LibrarySource, String> + Send + Sync;
static LIBRARY_RESOLVER: LazyLock<Mutex<Option<Arc<LibraryResolver>>>> =
    LazyLock::new(|| Mutex::new(None));
pub fn set_library_resolver(
    resolve: impl Fn(&str) -> Result<LibrarySource, String> + Send + Sync + 'static,
) {
    *LIBRARY_RESOLVER.lock().unwrap() = Some(Arc::new(resolve));
}
pub(crate) fn resolve_library(path: &str) -> Result<LibrarySource, String> {
    let resolver = LIBRARY_RESOLVER
        .lock()
        .map_err(|_| "Library unavailable")?
        .clone()
        .ok_or("Sign in to play cloud music")?;
    resolver(path)
}
