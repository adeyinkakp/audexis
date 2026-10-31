use super::StreamOptions;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::SystemTime,
};
static INDEXES: LazyLock<Mutex<HashMap<PathBuf, Index>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
#[derive(Default)]
struct Index {
    entries: BTreeMap<PathBuf, (u64, u64)>,
    ages: BTreeSet<(u64, PathBuf)>,
    clock: u64,
    bytes: u64,
}
impl Index {
    fn scan(root: &Path) -> Self {
        let mut index = Self::default();
        let Ok(directory) = fs::read_dir(root) else {
            return index;
        };
        let mut entries: Vec<_> = directory
            .flatten()
            .filter_map(|entry| {
                if entry.path().extension()?.to_str()? != "chunk" {
                    return None;
                }
                let meta = entry.metadata().ok()?;
                Some((meta.modified().ok()?, meta.len(), entry.path()))
            })
            .collect();
        entries.sort_by_key(|entry| entry.0);
        for (_, size, path) in entries {
            index.touch(path, size);
        }
        index
    }
    fn forget(&mut self, path: &Path) {
        if let Some((size, age)) = self.entries.remove(path) {
            self.bytes = self.bytes.saturating_sub(size);
            self.ages.remove(&(age, path.to_owned()));
        }
    }
    fn touch(&mut self, path: PathBuf, size: u64) {
        self.forget(&path);
        self.clock += 1;
        self.bytes += size;
        self.entries.insert(path.clone(), (size, self.clock));
        self.ages.insert((self.clock, path));
    }
    fn prune(&mut self, limit: u64) {
        while self.bytes > limit {
            let Some((_, path)) = self.ages.first().cloned() else {
                break;
            };
            if fs::remove_file(&path).is_err() && path.exists() {
                break;
            }
            self.forget(&path);
        }
    }
}
pub(super) struct Cache {
    root: PathBuf,
    key: String,
    limit: u64,
}
impl Cache {
    pub fn new(options: &StreamOptions) -> Option<Self> {
        if options.cache_bytes == 0 {
            return None;
        }
        let root = options.cache_dir.clone()?;
        let mut indexes = INDEXES.lock().ok()?;
        fs::create_dir_all(&root).ok()?;
        indexes
            .entry(root.clone())
            .or_insert_with(|| Index::scan(&root))
            .prune(options.cache_bytes);
        Some(Self {
            root,
            key: format!("{:x}", Sha256::digest(options.cache_key.as_bytes())),
            limit: options.cache_bytes,
        })
    }
    fn path(&self, start: u64) -> PathBuf {
        self.root.join(format!("{}-{start:016x}.chunk", self.key))
    }
    pub fn read(&self, start: u64, expected: usize) -> Option<Vec<u8>> {
        let mut indexes = INDEXES.lock().ok()?;
        let path = self.path(start);
        let mut bytes = Vec::new();
        let file = fs::File::open(&path).ok()?;
        (&file)
            .take(expected as u64 + 33)
            .read_to_end(&mut bytes)
            .ok()?;
        let index = indexes.get_mut(&self.root)?;
        if bytes.len() != expected + 32 || Sha256::digest(&bytes[32..]).as_slice() != &bytes[..32] {
            let _ = fs::remove_file(&path);
            index.forget(&path);
            return None;
        }
        let _ = file.set_modified(SystemTime::now());
        index.touch(path, bytes.len() as u64);
        Some(bytes.split_off(32))
    }
    pub fn write(&self, start: u64, bytes: &[u8]) {
        let Ok(mut indexes) = INDEXES.lock() else {
            return;
        };
        if bytes.len() as u64 + 32 > self.limit {
            return;
        }
        let temp = self.root.join(format!("{}.part", uuid::Uuid::new_v4()));
        let path = self.path(start);
        let result = (|| -> std::io::Result<()> {
            let mut file = fs::File::create(&temp)?;
            file.write_all(&Sha256::digest(bytes))?;
            file.write_all(bytes)?;
            file.flush()?;
            drop(file);
            fs::rename(&temp, &path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
            return;
        }
        if let Some(index) = indexes.get_mut(&self.root) {
            index.touch(path, bytes.len() as u64 + 32);
            index.prune(self.limit);
        }
    }
}
pub(super) fn clear(root: &Path) -> std::io::Result<()> {
    let mut indexes = INDEXES
        .lock()
        .map_err(|_| std::io::Error::other("Playback cache unavailable"))?;
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)?.flatten() {
        if matches!(
            entry.path().extension().and_then(|s| s.to_str()),
            Some("chunk" | "part")
        ) {
            fs::remove_file(entry.path())?;
        }
    }
    indexes.insert(root.to_owned(), Index::default());
    Ok(())
}
