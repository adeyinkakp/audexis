use super::*;

impl CloudSyncContext {
    pub async fn edit_track(&self, track: &Track, metadata: Metadata) -> Result<()> {
        store::enqueue_user(
            &self.db.pool,
            &self.connection_id,
            &format!("cloud-track:{}", track.id),
            &Job::Mutate {
                file_id: None,
                playlist_id: None,
                request: Mutation {
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    base_revision: track.revision,
                    edit: Edit::Track {
                        track_id: track.id.clone(),
                        metadata,
                    },
                },
            },
        )
        .await
    }
    pub async fn delete_track(&self, track: &Track) -> Result<()> {
        store::enqueue_user(
            &self.db.pool,
            &self.connection_id,
            &format!("cloud-track:{}", track.id),
            &Job::Mutate {
                file_id: None,
                playlist_id: None,
                request: Mutation {
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    base_revision: track.revision,
                    edit: Edit::DeleteTrack {
                        track_id: track.id.clone(),
                    },
                },
            },
        )
        .await
    }
    pub async fn save_playlist(&self, playlist: &Playlist) -> Result<()> {
        store::enqueue_user(
            &self.db.pool,
            &self.connection_id,
            &format!("cloud-playlist:{}", playlist.id),
            &Job::Mutate {
                file_id: None,
                playlist_id: None,
                request: Mutation {
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    base_revision: playlist.revision,
                    edit: Edit::Playlist {
                        playlist_id: playlist.id.clone(),
                        name: playlist.name.clone(),
                        track_ids: playlist.track_ids.clone(),
                    },
                },
            },
        )
        .await
    }
    pub async fn delete_playlist(&self, playlist: &Playlist) -> Result<()> {
        store::enqueue_user(
            &self.db.pool,
            &self.connection_id,
            &format!("cloud-playlist:{}", playlist.id),
            &Job::Mutate {
                file_id: None,
                playlist_id: None,
                request: Mutation {
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    base_revision: playlist.revision,
                    edit: Edit::DeletePlaylist {
                        playlist_id: playlist.id.clone(),
                    },
                },
            },
        )
        .await
    }
}
