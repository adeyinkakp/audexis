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
    <SongCollection
      key={JSON.stringify([
        queueInfo.queue_ids,
        queueInfo.paths,
        queueInfo.file_ids,
        queueInfo.occurrences,
      ])}
      label="Queue"
      disabled={removing}
      contextMenuItems={(entries) => [
        {
          text: "Remove from Queue",
          disabled: removing,
          action: async () => {
            if (removingRef.current) return;
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
          {virtualizer.getVirtualItems().map((item) => {
            const file = queueItems[item.index];
            const isCurrent = item.index === queueInfo.current_index;

            return (
              <SongCollectionItem
                itemKey={queueInfo.queue_ids[item.index]}
                fileId={file.id}
                key={queueInfo.queue_ids[item.index]}
                onPlay={() => onPlay(item.index)}
              >
                <div
                  key={`${file.id}-${file.occurrence}-${item.index}`}
                  className={`absolute left-0 flex w-full items-center gap-3 px-3 text-left hover:bg-muted/50 rounded-lg bg-blend-screen ${isCurrent ? "bg-muted/60 text-primary" : ""}`}
                  style={{
                    height: `${item.size}px`,
                    transform: `translateY(${item.start}px)`,
                  }}
                >
                  <div className="flex min-w-0 flex-1 items-center gap-3 text-left">
                    <div className="h-8 w-8 shrink-0 aspect-square overflow-hidden ">
                      <Artwork
                        round={false}
                        key={file.id}
                        id={file.id > 0 ? file.id : null}
                      />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-xs">{file.title}</div>
                      <div className="truncate text-[10px] text-muted-foreground">
                        {file.artist}
                      </div>
                    </div>
                    <span className="ml-auto text-[10px] tabular-nums text-muted-foreground">
                      {formatDuration(file.durationMs)}
                    </span>
                  </div>
                </div>
              </SongCollectionItem>
            );
          })}
        </div>
      </div>
    </SongCollection>
  );
}
