use bytes::Bytes;
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::{
    io::Cursor,
    num::NonZeroUsize,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const MAX_SOURCE: usize = 16 * 1024 * 1024;
const EDGE: u32 = 256;

pub fn thumbnail_edge(pixels: u32) -> u32 {
    [64, 96, 128, 192, EDGE]
        .into_iter()
        .find(|edge| *edge >= pixels)
        .unwrap_or(EDGE)
}

fn variant_hash(source: [u8; 32], edge: u32) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(source);
    hash.update(edge.to_le_bytes());
    hash.finalize().into()
}

#[derive(Clone)]
pub struct Artwork {
    pub hash: [u8; 32],
    pub bytes: Bytes,
}
struct CachedSource {
    loaded: Instant,
    hash: Option<[u8; 32]>,
}
struct Cache {
    sources: LruCache<(i64, String), CachedSource>,
    images: LruCache<[u8; 32], Arc<Artwork>>,
}
pub struct ArtworkCache {
    pool: sqlx::SqlitePool,
    cache: Mutex<Cache>,
}
impl ArtworkCache {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            pool,
            cache: Mutex::new(Cache {
                sources: LruCache::new(NonZeroUsize::new(1024).unwrap()),
                images: LruCache::new(NonZeroUsize::new(256).unwrap()),
            }),
        }
    }
    pub async fn load(&self, file_id: i64, path: String) -> Result<Option<Arc<Artwork>>, String> {
        self.load_sized(file_id, path, EDGE).await
    }
    pub fn cached(&self, file_id: i64, path: &str, pixels: u32) -> Option<Arc<Artwork>> {
        let mut cache = self.cache.try_lock().ok()?;
        let source = cache.sources.get(&(file_id, path.to_owned()))?;
        if source.loaded.elapsed() >= Duration::from_secs(300) {
            return None;
        }
        let hash = variant_hash(source.hash?, thumbnail_edge(pixels));
        cache.images.get(&hash).cloned()
    }
    pub async fn load_sized(
        &self,
        file_id: i64,
        path: String,
        pixels: u32,
    ) -> Result<Option<Arc<Artwork>>, String> {
        let edge = thumbnail_edge(pixels);
        let mut cache = self.cache.lock().await;
        let key = (file_id, path.clone());
        if let Some(source) = cache.sources.get(&key)
            && source.loaded.elapsed() < Duration::from_secs(300)
        {
            let hash = source.hash;
            match hash {
                None => return Ok(None),
                Some(hash) => {
                    if let Some(image) = cache.images.get(&variant_hash(hash, edge)).cloned() {
                        return Ok(Some(image));
                    }
                }
            }
        }
        let stored: Option<(Vec<u8>,)> = if file_id > 0 {
            sqlx::query_as(
                "SELECT data FROM metadata_pictures WHERE file_id=? AND length(data)<=?
                ORDER BY CASE WHEN picture_type=3 THEN 0 ELSE 1 END, id LIMIT 1",
            )
            .bind(file_id)
            .bind(MAX_SOURCE as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            None
        };
        let bytes = match stored {
            Some((bytes,)) => Some(bytes),
            None if !path.is_empty() && !path.starts_with("cloud-library://") => {
                tokio::task::spawn_blocking(move || embedded_cover(&path))
                    .await
                    .map_err(|e| e.to_string())??
            }
            None => None,
        };
        let Some(bytes) = bytes else {
            cache.sources.put(
                key,
                CachedSource {
                    loaded: Instant::now(),
                    hash: None,
                },
            );
            return Ok(None);
        };
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        let artwork = if let Some(artwork) = cache.images.get(&variant_hash(hash, edge)).cloned() {
            artwork
        } else {
            let bytes = tokio::task::spawn_blocking(move || thumbnail(bytes, edge))
                .await
                .map_err(|e| e.to_string())??;
            let artwork = Arc::new(Artwork {
                hash: variant_hash(hash, edge),
                bytes: bytes.into(),
            });
            cache.images.put(artwork.hash, artwork.clone());
            artwork
        };
        cache.sources.put(
            key,
            CachedSource {
                loaded: Instant::now(),
                hash: Some(hash),
            },
        );
        Ok(Some(artwork))
    }
    pub async fn invalidate_files(&self, ids: &[i64]) {
        let ids: std::collections::HashSet<_> = ids.iter().copied().collect();
        let mut cache = self.cache.lock().await;
        let keys: Vec<_> = cache
            .sources
            .iter()
            .filter(|(key, _)| ids.contains(&key.0))
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            cache.sources.pop(&key);
        }
    }
    pub async fn invalidate_source(&self, file_id: i64, path: &str) {
        self.cache
            .lock()
            .await
            .sources
            .pop(&(file_id, path.to_owned()));
    }
    pub async fn invalidate(&self) {
        self.cache.lock().await.sources.clear();
    }
}
fn thumbnail(bytes: Vec<u8>, edge: u32) -> Result<Vec<u8>, String> {
    if bytes.len() > MAX_SOURCE {
        return Err("Artwork exceeds 16 MB".into());
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| e.to_string())?
        .thumbnail(edge, edge)
        .to_rgba8();
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(output.into_inner())
}
fn embedded_cover(path: &str) -> Result<Option<Vec<u8>>, String> {
    use symphonia::core::{formats::probe::Hint, io::MediaSourceStream, meta::StandardVisualKey};
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            MediaSourceStream::new(Box::new(file), Default::default()),
            Default::default(),
            Default::default(),
        )
        .map_err(|e| e.to_string())?;
    let metadata = format.metadata();
    let Some(revision) = metadata.current() else {
        return Ok(None);
    };
    let visual = revision
        .media
        .visuals
        .iter()
        .filter(|visual| visual.data.len() <= MAX_SOURCE)
        .min_by_key(|visual| {
            if visual.usage == Some(StandardVisualKey::FrontCover) {
                0
            } else {
                1
            }
        });
    Ok(visual.map(|visual| visual.data.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn shares_album_art_and_invalidates_missing_entries() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE metadata_pictures (id INTEGER PRIMARY KEY, file_id INTEGER, data BLOB, picture_type INTEGER)")
            .execute(&pool).await.unwrap();
        let cache = ArtworkCache::new(pool.clone());
        assert!(cache.load(1, String::new()).await.unwrap().is_none());
        let mut png = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(512, 512, image::Rgba([224, 100, 100, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        for id in [1, 2] {
            sqlx::query("INSERT INTO metadata_pictures(file_id,data,picture_type) VALUES (?,?,3)")
                .bind(id)
                .bind(png.get_ref())
                .execute(&pool)
                .await
                .unwrap();
        }
        cache.invalidate().await;
        let first = cache.load(1, String::new()).await.unwrap().unwrap();
        let second = cache.load(2, String::new()).await.unwrap().unwrap();
        cache.invalidate_files(&[1]).await;
        assert!(cache.cached(1, "", EDGE).is_none());
        assert!(cache.cached(2, "", EDGE).is_some());
        let reloaded = cache.load(1, String::new()).await.unwrap().unwrap();
        assert!(Arc::ptr_eq(&first, &reloaded));

        assert_eq!(first.hash, second.hash);
        assert_eq!(first.bytes.as_ptr(), second.bytes.as_ptr());
        assert_eq!(image::load_from_memory(&first.bytes).unwrap().width(), EDGE);
        assert!(thumbnail(vec![1, 2, 3], EDGE).is_err());
        let small = cache
            .load_sized(1, String::new(), 70)
            .await
            .unwrap()
            .unwrap();
        let shared_small = cache
            .load_sized(2, String::new(), 90)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(image::load_from_memory(&small.bytes).unwrap().width(), 96);
        assert_ne!(
            first.hash, small.hash,
            "renderer keys must distinguish resolutions"
        );
        assert!(Arc::ptr_eq(&small, &shared_small));
        let small_weak = Arc::downgrade(&small);
        drop(small);
        drop(shared_small);
        assert!(small_weak.upgrade().is_some());
        let weak = Arc::downgrade(&first);
        drop(first);
        assert!(
            weak.upgrade().is_some(),
            "another visible consumer owns the cover"
        );
        drop(second);
        assert!(
            weak.upgrade().is_some(),
            "recent covers survive scrolling out of view"
        );
        let reloaded = cache.load(1, String::new()).await.unwrap().unwrap();
        assert_eq!(
            image::load_from_memory(&reloaded.bytes).unwrap().width(),
            EDGE
        );
        assert!(weak.upgrade().is_some());
    }
}
