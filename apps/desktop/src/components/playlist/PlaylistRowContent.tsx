import type { PlaylistTrack } from "../../hooks/usePlaylists";
import { formatDuration } from "../../utils/duration";
import { cn } from "../../utils";
import { titleFor, subtitleFor } from "./metadata";

export function PlaylistRowContent({
  track,
  metadata,
  isCurrent,
  isDragging,
}: {
  track: PlaylistTrack;
  index: number;
  metadata: { key: string; value: string }[] | undefined;
  isCurrent: boolean;
  isDragging?: boolean;
}) {
  return (
    <div
      className={cn(
        "grid grid-cols-[minmax(0,1fr)_64px] items-center gap-3 px-5 py-3 transition-[background-color,box-shadow,transform,opacity] duration-150",
        isDragging
          ? "rounded-xl bg-card shadow-xl ring-1 ring-primary/20 opacity-95"
          : isCurrent
            ? "bg-primary/8 text-primary hover:bg-primary/10"
            : "hover:bg-muted/35",
      )}
    >
      <div className="min-w-0 w-full flex-1 text-left">
        <div
          className={cn(
            "truncate w-full text-sm font-medium",
            isCurrent && "text-primary",
          )}
        >
          {titleFor(metadata, track.file_name)}
        </div>
        <div
          className={cn(
            "truncate text-xs",
            isCurrent ? "text-primary/75" : "text-muted-foreground",
          )}
        >
          {subtitleFor(metadata) || track.file_name}
        </div>
      </div>
      <span className="text-right text-xs tabular-nums text-muted-foreground">
        {formatDuration(track.duration_ms)}
      </span>
    </div>
  );
}
