import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import {
  useDeletePlaylist,
  usePlaylist,
  useRenamePlaylist,
} from "../hooks/usePlaylists";
import { PlaylistHeader } from "./playlist/PlaylistHeader";

import { PlaylistTrackList } from "./playlist/PlaylistTrackList";

export default function PlaylistEditor({
  playlistId,
  onDeleted,
  search = "",
}: {
  playlistId: number;
  search?: string;
  onDeleted?: () => void;
}) {
  const { data: playlist } = usePlaylist(playlistId);
  const renamePlaylist = useRenamePlaylist();
  const deletePlaylist = useDeletePlaylist();
  const [playlistError, setPlaylistError] = useState<string | null>(null);

  const playQueue = async (startIndex = 0, shuffle = false) => {
    if (!playlist || playlist.tracks.length === 0) return;

    const queue = playlist.tracks.map((track) => ({
      id: track.id,
      path: track.path,
      occurrence: track.ord,
    }));
    const curr = queue[Math.min(startIndex, queue.length - 1)];

    await invoke("play_song", {
      playbackInfo: {
        curr,
        queue,
        currentIndex: startIndex,
        playlistId: playlist.id,
      },
    });

    if (shuffle) {
      await invoke("toggle_shuffle");
    }
  };

  if (!playlist) {
    return (
      <div className="space-y-6">
        <section className="rounded-3xl border border-border/60 bg-card/40 p-6">
          <div className="text-sm text-muted-foreground">
            Playlist not found.
          </div>
        </section>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <PlaylistHeader
        playlist={playlist}

        onRename={(name) =>
          renamePlaylist.mutateAsync({ id: playlist.id, name })
        }
        onPlay={(shuffle) => {
          void playQueue(0, shuffle);
        }}
        onDelete={() => {
          void deletePlaylist
            .mutateAsync(playlist.id)
            .then(() => onDeleted?.());
        }}
      />
      {playlistError && (
        <div className="rounded-xl border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
          {playlistError}
        </div>
      )}

      <PlaylistTrackList
        search={search}
        playlist={playlist}
        playQueue={playQueue}
        onError={setPlaylistError}
      />
    </div>
  );
}
