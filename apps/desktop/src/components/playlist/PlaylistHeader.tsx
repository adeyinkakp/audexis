import { useState } from "react";
import { PlaylistNameModal } from "../../modals/PlaylistNameModal";
import { Music2, Pencil, Play, Shuffle } from "lucide-react";
import { formatDurationInWords } from "../../utils/duration";
import type { PlaylistDetail } from "../../hooks/usePlaylists";

export function PlaylistHeader({
  playlist,

  onRename,
  onPlay,
  onDelete,
}: {
  playlist: PlaylistDetail;

  onRename: (name: string) => Promise<unknown>;
  onPlay: (shuffle: boolean) => void;
  onDelete: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const totalDurationMs = playlist.tracks.reduce(
    (total, track) =>
      total +
      (track.duration_ms && track.duration_ms > 0 ? track.duration_ms : 0),
    0,
  );
  const hasMissingDurations = playlist.tracks.some(
    (track) => track.duration_ms == null || track.duration_ms <= 0,
  );

  return (
    <section className="  p-6 ">
      <div className="flex flex-col gap-6 lg:flex-row lg:items-end items-center">
        <div className="flex h-52 w-52 shrink-0 items-center justify-center rounded-3xl bg-muted shadow-lg">
          <Music2 size={56} className="text-muted-foreground" />
        </div>

        <div className="min-w-0 flex-1 flex lg:block flex-col items-center justify-center b">
          <p className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            Playlist
          </p>
          <span className="text-4xl font-semibold tracking-tight outline-none md:text-5xl">
            {playlist.name}
          </span>
          <p className="mt-3 text-sm text-muted-foreground">
            {playlist.tracks.length}{" "}
            {playlist.tracks.length === 1 ? "song" : "songs"}
            {" · "}
            {hasMissingDurations && totalDurationMs === 0
              ? "—"
              : formatDurationInWords(totalDurationMs)}
            {hasMissingDurations ? " duration incomplete" : " total"}
          </p>

          <div className="mt-6 flex flex-wrap items-center gap-3">
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-full bg-primary px-5 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
              disabled={playlist.tracks.length === 0}
              onClick={() => onPlay(false)}
            >
              <Play size={16} fill="currentColor" /> Play
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-full bg-muted px-5 py-2.5 text-sm font-medium transition-colors hover:bg-muted/80 disabled:opacity-50"
              disabled={playlist.tracks.length === 0}
              onClick={() => onPlay(true)}
            >
              <Shuffle size={16} /> Shuffle
            </button>
            <button
              type="button"
              onClick={() => setEditing(true)}
              className="inline-flex items-center gap-2 rounded-full border border-border px-4 py-2.5 text-sm font-medium hover:bg-muted"
            >
              <Pencil size={15} />
              Edit Playlist
            </button>
            <button
              type="button"
              className="rounded-full border border-destructive/30 px-4 py-2 text-sm text-destructive transition-colors hover:bg-destructive/10"
              onClick={() => {
                onDelete();
              }}
            >
              Delete
            </button>
          </div>
        </div>
      </div>
      {editing && (
        <PlaylistNameModal
          editing
          initialName={playlist.name}
          onClose={() => setEditing(false)}
          onSave={onRename}
        />
      )}
    </section>
  );
}
