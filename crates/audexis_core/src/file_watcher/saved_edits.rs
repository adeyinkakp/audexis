use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::SystemTime,
};

#[derive(PartialEq, Eq)]
pub(crate) struct Stamp {
    size: u64,
    modified: SystemTime,
    #[cfg(unix)]
    identity: (u64, u64),
}

impl Stamp {
    pub(crate) fn read(path: &Path) -> Option<Self> {
        let metadata = crate::library_files::metadata(path).ok()?;
        Some(Self {
            size: metadata.len(),
            modified: metadata.modified().ok()?,
            #[cfg(unix)]
            identity: {
                use std::os::unix::fs::MetadataExt;
                (metadata.dev(), metadata.ino())
            },
        })
    }
}

static SAVED: LazyLock<Mutex<lru::LruCache<PathBuf, Stamp>>> =
    LazyLock::new(|| Mutex::new(lru::LruCache::new(NonZeroUsize::new(4096).unwrap())));

pub(crate) fn remember(path: &Path, stamp: Option<Stamp>) {
    if let Some(stamp) = stamp
        && Stamp::read(path).as_ref() == Some(&stamp)
        && let Ok(mut saved) = SAVED.lock()
    {
        saved.put(path.to_path_buf(), stamp);
    }
}

pub(super) fn matches(path: &Path) -> bool {
    let current = Stamp::read(path);
    let Ok(mut saved) = SAVED.lock() else {
        return false;
    };
    if let Some(stamp) = saved.get(path) {
        if current.as_ref() == Some(stamp) {
            return true;
        }
        saved.pop(path);
    }
    false
}
