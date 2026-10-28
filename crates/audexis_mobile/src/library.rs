use audexis_core::database::{Database, songs::SongSort};
use serde_json::{Value, json};
use std::path::Path;
use tokio::runtime::{Builder, Runtime};

pub(super) struct CoreLibrary {
    pub(super) database: Database,
    pub(super) runtime: Runtime,
}

impl CoreLibrary {
    pub(super) fn open(directory: &Path) -> Result<Self, String> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        let database = runtime
            .block_on(Database::init(&directory.join("library.db")))
            .map_err(|error| error.to_string())?;
        let library = Self { database, runtime };
        library.initialize()?;
        Ok(library)
    }

    pub(super) fn songs(&self, search: &str, offset: usize) -> Result<Value, String> {
        let page = self
            .runtime
            .block_on(
                self.database
                    .songs_page(search, SongSort::Title, false, offset),
            )
            .map_err(|error| error.to_string())?;
        let songs: Vec<_> = page
            .rows
            .into_iter()
            .map(|song| {
                json!({
                    "id": song.id,
                    "title": song.title,
                    "artist": song.artist,
                    "album": song.album,
                    "path": song.path,
                    "download_path": song.download_path,
                    "duration_ms": song.duration_ms,
                    "format": song.format,
                })
            })
            .collect();
        Ok(json!({"songs": songs, "total": page.total, "offset": offset}))
    }
}

impl Drop for CoreLibrary {
    fn drop(&mut self) {
        self.runtime.block_on(self.database.pool.close());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MobileCore;

    #[test]
    fn c_bridge_returns_core_song_page() {
        use crate::ffi::*;
        use std::ffi::{CStr, CString};

        let directory =
            std::env::temp_dir().join(format!("audexis-mobile-{}", uuid::Uuid::new_v4()));
        let handle = audexis_mobile_create();
        let call = |request: Value| {
            let request = CString::new(request.to_string()).unwrap();
            // This test owns the handle and serializes all calls and destruction.
            unsafe {
                let response = audexis_mobile_call(handle, request.as_ptr());
                let json = CStr::from_ptr(response).to_str().unwrap().to_owned();
                audexis_mobile_string_free(response);
                serde_json::from_str::<Value>(&json).unwrap()
            }
        };
        let opened = call(json!({"op": "open", "directory": directory}));
        let page = call(json!({"op": "songs"}));
        unsafe { audexis_mobile_destroy(handle) };
        assert!(opened.get("ok").is_some(), "{opened}");
        assert_eq!(page, json!({"ok": {"songs": [], "total": 0, "offset": 0}}));
        assert!(directory.join("library.db").is_file());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn mobile_opens_core_database_and_reads_persisted_songs() {
        let directory =
            std::env::temp_dir().join(format!("audexis-mobile-{}", uuid::Uuid::new_v4()));
        let mut mobile = MobileCore::default();
        assert!(mobile.call(json!({"op": "songs"})).is_err());
        mobile
            .call(json!({"op": "open", "directory": directory}))
            .unwrap();
        assert!(directory.join("library.db").is_file());
        assert_eq!(mobile.call(json!({"op": "songs"})).unwrap()["total"], 0);
        let library = mobile.library.as_ref().unwrap();
        library.runtime.block_on(async {
            sqlx::query("INSERT INTO files(path,file_name,status,size) VALUES ('/music/song.mp3','song.mp3','ok',123)")
                .execute(&library.database.pool).await.unwrap();
            sqlx::query("INSERT INTO metadata_texts(file_id,key,value) SELECT id,'title','Hello from core' FROM files")
                .execute(&library.database.pool).await.unwrap();
        });
        drop(mobile);

        let mut reopened = MobileCore::default();
        reopened
            .call(json!({"op": "open", "directory": directory}))
            .unwrap();
        let page = reopened
            .call(json!({"op": "songs", "search": "Hello"}))
            .unwrap();
        assert_eq!(page["total"], 1);
        assert_eq!(page["songs"][0]["title"], "Hello from core");
        assert_eq!(
            reopened
                .call(json!({"op": "songs", "offset": 100}))
                .unwrap()["songs"],
            json!([])
        );
        assert_eq!(
            reopened
                .call(json!({"op": "songs", "search": "missing"}))
                .unwrap()["total"],
            0
        );
        assert!(reopened.call(json!({"op": "songs", "offset": -1})).is_err());
        drop(reopened);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
