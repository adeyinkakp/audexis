import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import { ListMusic, MessageSquareText, X } from "lucide-react";
import { useEffect, useMemo } from "react";
import QueuePanel from "./QueuePanel";
import LyricsPreview from "./info/LyricsPreview";
import { parseLyrics } from "./info/synchronizedLyrics";
import { useStore } from "../hooks/useStore";
import { cn } from "../utils";

export type PlaybackPanelView = "queue" | "lyrics";
export default function PlaybackPanel({
  view,
  onViewChange,
  onClose,
  onPlay,
  fileId,
  title,
  artist,
  artwork,
  position,
  onSeek,
}: {
  view: PlaybackPanelView;
  onViewChange: (view: PlaybackPanelView) => void;
  onClose: () => void;
  onPlay: (index: number) => void;
  fileId: number;
  title?: string;
  artist?: string;
  artwork?: string | null;
  position: number;
  onSeek: (milliseconds: number) => void;
}) {
  useEffect(() => {
    document.documentElement.style.setProperty("--queue-width", "20rem");
    return () => {
      document.documentElement.style.removeProperty("--queue-width");
    };
  }, []);
  return (
    <aside
      aria-label="Playback sidebar"
      className="fixed bottom-0 right-0 top-14 -z-40 flex w-80 flex-col overflow-hidden border-l border-border bg-popover shadow-xl"
    >
      <div className="flex h-12 shrink-0 items-center gap-1 border-b border-border px-3">
        {(
          [
            ["queue", "Queue", ListMusic],
            ["lyrics", "Lyrics", MessageSquareText],
          ] as const
        ).map(([id, label, Icon]) => (
          <button
            key={id}
            type="button"
            aria-pressed={view === id}
            onClick={() => onViewChange(id)}
            className={cn(
              "flex items-center gap-2 rounded-lg px-3 py-2 text-xs font-semibold text-muted-foreground hover:bg-muted",
              view === id && "bg-muted text-primary",
            )}
          >
            <Icon size={15} />
            {label}
          </button>
        ))}
        <button
          type="button"
          aria-label="Close playback sidebar"
          onClick={onClose}
          className="ml-auto rounded-lg p-2 text-muted-foreground hover:bg-muted"
        >
          <X size={16} />
        </button>
      </div>
      {view === "queue" ? (
        <QueuePanel onPlay={onPlay} />
      ) : (
        <SidebarLyrics
          key={fileId}
          fileId={fileId}
          title={title}
          artist={artist}
          artwork={artwork}
          position={position}
          onSeek={onSeek}
        />
      )}
    </aside>
  );
}

function SidebarLyrics({
  fileId,
  title,
  artist,
  artwork,
  position,
  onSeek,
}: {
  fileId: number;
  title?: string;
  artist?: string;
  artwork?: string | null;
  position: number;
  onSeek: (milliseconds: number) => void;
}) {
  const { openTrackInfo } = useStore();
  const query = useQuery({
    queryKey: ["lyrics", [fileId]],
    queryFn: () =>
      invoke<{ fileId: number; plain: string[]; synced: string[] }[]>(
        "get_lyrics",
        { fileIds: [fileId] },
      ),
    enabled: fileId > 0,
    staleTime: 0,
  });
  const lyric = query.data?.find((item) => item.fileId === fileId);
  const parsed = useMemo(() => parseLyrics(lyric?.synced[0] ?? ""), [lyric]);
  if (!fileId)
    return (
      <p className="p-6 text-sm text-muted-foreground">
        Play a song to see its lyrics.
      </p>
    );
  return (
    <section
      aria-label="Current song lyrics"
      className="relative isolate flex min-h-0 flex-1 flex-col overflow-hidden bg-[#211b27] text-white"
    >
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 -z-10 bg-[radial-gradient(ellipse_at_top_left,#754853,transparent),radial-gradient(ellipse_at_bottom_right,#4c3c61,transparent)]"
      >
        {artwork && (
          <img
            src={artwork}
            alt=""
            className="size-full scale-125 object-cover opacity-40 blur-3xl"
          />
        )}
        <div className="absolute inset-0 bg-black/25" />
      </div>
      <div className="shrink-0 px-6 pb-2 pt-6">
        <h2 className="truncate text-sm font-semibold">
          {title || "Current song"}
        </h2>
        <p className="mt-1 truncate text-xs text-white/50">
          {artist || "Unknown artist"}
        </p>
      </div>
      {query.isPending ? (
        <p role="status" className="p-6 text-sm text-white/60">
          Loading lyrics…
        </p>
      ) : query.isError ? (
        <div role="alert" className="p-6 text-sm text-white/60">
          Could not load lyrics.{" "}
          <button
            type="button"
            onClick={() => void query.refetch()}
            className="underline"
          >
            Retry
          </button>
        </div>
      ) : (
        <LyricsPreview
          rows={parsed.error ? [] : parsed.rows}
          plainText={lyric?.plain[0]}
          position={position}
          onSeek={onSeek}
          compact
          className="flex-1"
        />
      )}
      <button
        type="button"
        onClick={() => openTrackInfo([fileId])}
        className="mx-6 mb-5 mt-auto shrink-0 rounded-full bg-white/10 px-4 py-2 text-xs font-medium text-white/70 hover:bg-white/20"
      >
        Edit lyrics in Track Info
      </button>
    </section>
  );
}
