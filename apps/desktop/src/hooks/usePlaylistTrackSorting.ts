import {
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { useRef, useState } from "react";
import {
  useReorderPlaylistTrack,
  type PlaylistTrack,
  type PlaylistDetail,
} from "./usePlaylists";

export function usePlaylistTrackSorting(
  playlist: PlaylistDetail,
  onError: (error: string | null) => void,
) {
  const reorderPlaylistTrack = useReorderPlaylistTrack();
  const [activeOrd, setActiveOrd] = useState<number | null>(null);
  const [optimisticTracks, setOptimisticTracks] = useState<
    PlaylistTrack[] | null
  >(null);
  const saving = useRef(false);
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );
  const orderedTracksPreview = optimisticTracks ?? playlist.tracks;
  const sortableTrackIds = orderedTracksPreview.map((track) => track.ord);
  const activeTrack =
    playlist.tracks.find((track) => track.ord === activeOrd) ?? null;

  const handleDragStart = (event: DragStartEvent) => {
    if (!saving.current) setActiveOrd(Number(event.active.id));
  };

  const handleDragEnd = async ({ active, over }: DragEndEvent) => {
    setActiveOrd(null);
    if (saving.current || !over || active.id === over.id) return;

    const fromIndex = playlist.tracks.findIndex(
      (track) => track.ord === Number(active.id),
    );
    const toIndex = playlist.tracks.findIndex(
      (track) => track.ord === Number(over.id),
    );
    if (fromIndex < 0 || toIndex < 0) return;

    saving.current = true;
    setOptimisticTracks(arrayMove(playlist.tracks, fromIndex, toIndex));
    try {
      await reorderPlaylistTrack.mutateAsync({
        playlistId: playlist.id,
        fromOrd: playlist.tracks[fromIndex].ord,
        toOrd: playlist.tracks[toIndex].ord,
      });
      onError(null);
    } catch (error) {
      onError(String(error));
    } finally {
      setOptimisticTracks(null);
      saving.current = false;
    }
  };

  const handleDragCancel = () => setActiveOrd(null);
  return {
    sensors,
    orderedTracksPreview,
    sortableTrackIds,
    activeTrack,
    isSaving: reorderPlaylistTrack.isPending,
    handleDragStart,
    handleDragEnd,
    handleDragCancel,
  };
}
