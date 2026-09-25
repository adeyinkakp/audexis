import { DndContext, DragOverlay, closestCenter } from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import {
  useRemovePlaylistTrack,
  type PlaylistDetail,
} from "../../hooks/usePlaylists";
import { useCurrentQueueTrack } from "../../hooks/useCurrentQueueTrack";
import { usePlaylistTrackSorting } from "../../hooks/usePlaylistTrackSorting";
import { useMediaFiles } from "../../hooks/useMediaFiles";
import { useMemo } from "react";
import type { DatabaseMediaMetadata } from "../../hooks/useFileWatcher";
import { useNowPlayingState } from "../../hooks/useNowPlayingState";
import { PlaylistRowContent } from "./PlaylistRowContent";
import { SortablePlaylistRow } from "./SortablePlaylistRow";

export function PlaylistTrackList({
  playlist,
  search = "",
  playQueue,
  onError,
}: {
  playlist: PlaylistDetail;
  search?: string;
  playQueue: (startIndex: number, shuffle: boolean) => Promise<void>;
  onError: (error: string | null) => void;
}) {
  const { data: media } = useMediaFiles(
    playlist.tracks.map((track) => track.id),
  );
  const metadataByFileId = useMemo(() => {
    const result: Record<number, DatabaseMediaMetadata[]> = {};
    for (const item of media?.metadata ?? [])
      (result[item.file_id] ??= []).push(item);
    return result;
  }, [media]);
  const { currentTrackId, song } = useNowPlayingState();
  const removePlaylistTrack = useRemovePlaylistTrack();
  const currentQueueTrack = useCurrentQueueTrack();
  const {
    sensors,
    orderedTracksPreview,
    activeTrack,
    handleDragStart,
    isSaving,
    handleDragEnd,
    handleDragCancel,
  } = usePlaylistTrackSorting(playlist, onError);

  const terms = search.toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const matches = (track: PlaylistDetail["tracks"][number]) => {
    const values = [
      track.file_name,
      track.path,
      ...(metadataByFileId[track.id] ?? []).map((item) => item.value),
    ].map((value) => value.toLocaleLowerCase());
    return terms.every((term) => values.some((value) => value.includes(term)));
  };
  return (
    <section
      aria-busy={isSaving}
      className={`overflow-hidden rounded-3xl] border border-border/60 bg-card/40 ${isSaving ? "pointer-events-none" : ""}`}
    >
      <div className="grid grid-cols-[32px_minmax(0,1fr)_64px] gap-3 border-b border-border/60 px-5 py-3 text-xs uppercase tracking-[0.16em] text-muted-foreground">
        <div />

        <div>Title</div>
        <div className="text-right">Time</div>
      </div>

      {!orderedTracksPreview.some(matches) ? (
        <div className="px-5 py-8 text-sm text-muted-foreground">
          {search.trim()
            ? "No matching songs in this playlist."
            : "No songs in this playlist."}
        </div>
      ) : (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
          onDragCancel={handleDragCancel}
        >
          <SortableContext
            items={orderedTracksPreview
              .filter(matches)
              .map((track) => track.ord)}
            strategy={verticalListSortingStrategy}
          >
            {orderedTracksPreview.map((track, index) => {
              if (!matches(track)) return null;
              const metadata = metadataByFileId[track.id];
              const isCurrent =
                track.id === currentTrackId &&
                !!currentQueueTrack &&
                currentQueueTrack.path === track.path &&
                currentQueueTrack.occurrence === track.ord;
              const fallbackIsCurrent =
                !currentQueueTrack &&
                (track.id === currentTrackId ||
                  (!!song?.path && song.path === track.path));

              return (
                <SortablePlaylistRow
                  key={`${track.id}-${track.ord}`}
                  track={track}
                  playlistId={playlist.id}
                  index={index}
                  metadata={metadata}
                  isCurrent={isCurrent || fallbackIsCurrent}
                  onPlay={() => void playQueue(index, false)}
                  onRemove={() => {
                    void removePlaylistTrack
                      .mutateAsync({
                        playlistId: playlist.id,
                        ord: track.ord,
                      })
                      .then(() => onError(null))
                      .catch((error) => {
                        onError(String(error));
                      });
                  }}
                />
              );
            })}
          </SortableContext>
          <DragOverlay>
            {activeTrack ? (
              <div className="rounded-xl border border-border/70 bg-popover/95 shadow-2xl backdrop-blur">
                <PlaylistRowContent
                  track={activeTrack}
                  index={orderedTracksPreview.findIndex(
                    (track) => track.ord === activeTrack.ord,
                  )}
                  metadata={metadataByFileId[activeTrack.id]}
                  isCurrent={
                    (!!currentQueueTrack &&
                      currentTrackId === activeTrack.id &&
                      currentQueueTrack.path === activeTrack.path &&
                      currentQueueTrack.occurrence === activeTrack.ord) ||
                    (!currentQueueTrack &&
                      (activeTrack.id === currentTrackId ||
                        (!!song?.path && song.path === activeTrack.path)))
                  }

                  isDragging
                  onPlay={() => {}}
                />
              </div>
            ) : null}
          </DragOverlay>
        </DndContext>
      )}
    </section>
  );
}
