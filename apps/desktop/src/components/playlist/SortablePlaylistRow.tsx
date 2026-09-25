import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { PlaylistTrack } from "../../hooks/usePlaylists";
import { SongContextMenu } from "../SongContextMenu";
import { cn } from "../../utils";
import { PlaylistRowContent } from "./PlaylistRowContent";

export function SortablePlaylistRow({
  track,
  index,
  metadata,
  isCurrent,
  onPlay,
  onRemove,
  playlistId,
}: {
  playlistId: number;
  track: PlaylistTrack;
  index: number;
  metadata: { key: string; value: string }[] | undefined;
  isCurrent: boolean;
  onPlay: () => void;
  onRemove: () => void;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: track.ord });
  return (
    <SongContextMenu
      fileId={track.id}
      playlistId={playlistId}
      onRemoveFromPlaylist={onRemove}
      asChild={false}
    >
      <div
        ref={setNodeRef}
        style={{
          transform: CSS.Transform.toString(transform),
          transition:
            transition ?? "transform 180ms cubic-bezier(0.2, 0, 0, 1)",
          zIndex: isDragging ? 10 : undefined,
        }}
        className={cn(isDragging && "opacity-0")}
      >
        <PlaylistRowContent
          track={track}
          index={index}
          metadata={metadata}
          isCurrent={isCurrent}
          isDragging={isDragging}
          dragHandleAttributes={attributes}
          dragHandleListeners={listeners}
          onPlay={onPlay}
        />
      </div>
    </SongContextMenu>
  );
}
