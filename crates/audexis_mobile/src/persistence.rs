use super::*;

impl MobileCore {
    pub(super) fn root(&self) -> Result<&Path, String> {
        self.root.as_deref().ok_or("Library is not open".into())
    }
    pub(super) fn save(&self) -> Result<(), String> {
        let root = self.root()?;
        let bytes = serde_json::to_vec(&self.state).map_err(|e| e.to_string())?;
        let temporary = root.join("library.json.tmp");
        fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
        fs::rename(temporary, root.join("library.json")).map_err(|e| e.to_string())
    }
    pub(super) fn snapshot(&self) -> Value {
        let mut tracks: Vec<_> = self.state.tracks.values().collect();
        tracks.sort_by_cached_key(|t| {
            (
                t.title().to_lowercase(),
                t.artist().to_lowercase(),
                t.id.clone(),
            )
        });
        let downloads: BTreeMap<_, _> = self
            .state
            .downloads
            .iter()
            .filter(|(id, d)| {
                self.state
                    .tracks
                    .get(*id)
                    .is_some_and(|t| t.sha256 == d.source_hash)
                    && self
                        .root
                        .as_ref()
                        .is_some_and(|r| r.join("audio").join(&d.file).is_file())
            })
            .map(|(id, d)| {
                (
                    id,
                    self.root
                        .as_ref()
                        .unwrap()
                        .join("audio")
                        .join(&d.file)
                        .to_string_lossy()
                        .into_owned(),
                )
            })
            .collect();
        json!({"cursor":self.state.cursor,"tracks":tracks,"playlists":self.state.playlists.values().collect::<Vec<_>>(),
               "queue":self.state.queue,"current":self.state.current,"downloads":downloads})
    }
    pub(super) fn track(&self, id: &str) -> Result<&Track, String> {
        self.state
            .tracks
            .get(id)
            .ok_or("This cloud track is no longer available".into())
    }
}
