import {
  DndContext,
  DragOverlay,
  PointerSensor,
  KeyboardSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
  sortableKeyboardCoordinates,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import toast from "react-hot-toast";
import { Artwork } from "./library/CollectionGrid";
import { SongCollection, SongCollectionItem } from "./songs/SongCollection";
import { formatDuration } from "../utils/duration";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useMemo, useRef, useState } from "react";
import { useMediaFiles } from "../hooks/useMediaFiles";

type QueueInfo = {
  queue_ids: string[];
  paths: string[];
  file_ids: number[];
  occurrences: (number | null)[];
  current_index: number;
  playlist_id: number | null;
};

type QueueItem = {
  id: number;
  path: string;
  occurrence: number | null;
  fileName: string;
  title: string;
  durationMs: number | null;
  artist: string;
};

export default function QueuePanel({
  onPlay,
}: {
  onPlay: (index: number) => void;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [removing, setRemoving] = useState(false);
  const removingRef = useRef(false);
  const [sorting, setSorting] = useState(false);
  const sortingRef = useRef(false);
  const [activeItem, setActiveItem] = useState<QueueItem | null>(null);
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
      keyboardCodes: { start: ["Space"], cancel: ["Escape"], end: ["Space", "Enter"] },
    }),
  );
  const handleDragEnd = async ({ active, over }: DragEndEvent) => {
    setActiveItem(null);
    if (sortingRef.current || removingRef.current || !over || active.id === over.id) return;
    sortingRef.current = true;
    setSorting(true);
    try {
      await invoke("reorder_queue", { queueId: String(active.id), targetId: String(over.id) });
    } catch (error) {
      toast.error(`Could not reorder queue: ${String(error)}`);
    } finally {
      sortingRef.current = false;
      setSorting(false);
    }
  };
  const [queueInfo, setQueueInfo] = useState<QueueInfo>({
    queue_ids: [],
    paths: [],
    file_ids: [],
    occurrences: [],
    current_index: 0,
    playlist_id: null,
  });

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    let disposed = false;
    let receivedEvent = false;

    void invoke<QueueInfo>("get_queue").then((result) => {
      if (!disposed && !receivedEvent) setQueueInfo(result);
    });
    void listen<QueueInfo>("queue-changed", (event) => {
      receivedEvent = true;
      if (!disposed) setQueueInfo(event.payload);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else cleanup = unlisten;
    });

    return () => {
      disposed = true;
      cleanup?.();
    };
  }, []);

  const virtualizer = useVirtualizer({
    count: queueInfo.paths.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 52,
    getItemKey: (index) => queueInfo.queue_ids[index],
    overscan: 8,
  });
  const visibleRows = virtualizer.getVirtualItems();
  const { data } = useMediaFiles(
    visibleRows.map((row) => queueInfo.file_ids[row.index]),
  );
  const queueItems = useMemo<QueueItem[]>(() => {
    const files = new Map(data?.files.map((file) => [file.id, file]));
    const titles = new Map(
      data?.metadata
        .filter((item) => item.key === "title")
        .map((item) => [item.file_id, item.value]),
    );
    const artists = new Map(
      data?.metadata
        .filter((item) => item.key === "artist")
        .map((item) => [item.file_id, item.value]),
    );
    return queueInfo.paths.map((path, index) => {
      const id = queueInfo.file_ids[index];
      const file = files.get(id);
      const fileName = file?.file_name ?? path.split(/[\\/]/).pop() ?? path;
      return {
        id,
        path,
        occurrence: queueInfo.occurrences[index] ?? null,
        fileName,
        title: titles.get(id) || fileName,
        artist: artists.get(id) || path,
        durationMs: file?.duration_ms ?? null,
      };
    });
  }, [data, queueInfo]);

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragStart={({ active }) => {
        if (!sortingRef.current && !removingRef.current) {
          setActiveItem(queueItems[queueInfo.queue_ids.indexOf(String(active.id))] ?? null);
        }
      }}
      onDragEnd={handleDragEnd}
      onDragCancel={() => setActiveItem(null)}
    >
    <SongCollection
      key={JSON.stringify([...queueInfo.queue_ids].sort())}
      label="Queue"
      disabled={removing || sorting}
      contextMenuItems={(entries) => [
        {
          text: "Remove from Queue",
          disabled: removing || sorting || activeItem !== null,
          action: async () => {
            if (removingRef.current || sortingRef.current || activeItem) return;
            removingRef.current = true;
            setRemoving(true);
            try {
              await invoke("remove_queue_entries", {
                queueIds: entries.map((entry) => entry.key),
              });
            } catch (error) {
              toast.error(
                `Could not remove songs from queue: ${String(error)}`,
              );
            } finally {
              removingRef.current = false;
              setRemoving(false);
            }
          },
        },
      ]}
      items={queueItems.map((file, index) => ({
        key: queueInfo.queue_ids[index],
        fileId: file.id,
      }))}
      className="flex min-h-0 flex-1 flex-col"
    >
      <div ref={scrollRef} className="min-h-0 flex-1 overflow-auto">
        <div
          className="relative"
          style={{ height: `${virtualizer.getTotalSize()}px` }}
        >
          <SortableContext items={queueInfo.queue_ids} strategy={verticalListSortingStrategy}>
            {visibleRows.map((item) => (
              <SortableQueueRow
                key={queueInfo.queue_ids[item.index]}
                queueId={queueInfo.queue_ids[item.index]}
                file={queueItems[item.index]}
                isCurrent={item.index === queueInfo.current_index}
                start={item.start}
                size={item.size}
                disabled={removing || sorting}
                onPlay={() => onPlay(item.index)}
              />
            ))}
          </SortableContext>
        </div>
      </div>
    </SongCollection>
    <DragOverlay dropAnimation={null}>
      {activeItem ? (
        <div className="flex h-[52px] items-center gap-2 rounded-lg border border-border bg-popover px-3 shadow-xl">
          <QueueRowContent file={activeItem} />
        </div>
      ) : null}
    </DragOverlay>
    </DndContext>
  );
}

function QueueRowContent({ file }: { file: QueueItem }) {
  return (
    <div className="flex min-w-0 flex-1 items-center gap-3 text-left">
      <div className="h-8 w-8 shrink-0 overflow-hidden">
        <Artwork round={false} id={file.id > 0 ? file.id : null} />
      </div>
      <div className="min-w-0 flex-1">
        <div className="truncate text-xs">{file.title}</div>
        <div className="truncate text-[10px] text-muted-foreground">{file.artist}</div>
      </div>
      <span className="ml-auto text-[10px] tabular-nums text-muted-foreground">
        {formatDuration(file.durationMs)}
      </span>
    </div>
  );
}

function SortableQueueRow({ queueId, file, isCurrent, start, size, disabled, onPlay }: {
  queueId: string;
  file: QueueItem;
  isCurrent: boolean;
  start: number;
  size: number;
  disabled: boolean;
  onPlay: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: queueId, disabled });
  return (
    <SongCollectionItem itemKey={queueId} fileId={file.id} onPlay={onPlay}>
      <div
        ref={setNodeRef}
        {...attributes}
        {...listeners}
        className={`touch-none cursor-grab active:cursor-grabbing absolute left-0 flex w-full items-center gap-2 rounded-lg px-3 text-left hover:bg-muted/50 ${isCurrent ? "bg-muted/60 text-primary" : ""}`}
        style={{ top: start, height: size, transform: CSS.Transform.toString(transform), transition, opacity: isDragging ? 0 : 1 }}
      >
        <QueueRowContent file={file} />
      </div>
    </SongCollectionItem>
  );
}
