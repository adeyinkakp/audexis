import { SongContextMenu } from "./SongContextMenu";
import { formatDuration } from "../utils/duration";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ListMusic } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useMediaFiles } from "../hooks/useMediaFiles";

type QueueInfo = {
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
};

export default function QueuePanel({
  onPlay,
}: {
  onPlay: (index: number) => void;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [queueInfo, setQueueInfo] = useState<QueueInfo>({
    paths: [],
    file_ids: [],
    occurrences: [],
    current_index: 0,
    playlist_id: null,
  });

  useEffect(() => {
    document.documentElement.style.setProperty("--queue-width", "20rem");
    return () => {
      document.documentElement.style.removeProperty("--queue-width");
    };
  }, []);

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
        durationMs: file?.duration_ms ?? null,
      };
    });
  }, [data, queueInfo]);

  return (
    <aside className="fixed right-4 top-0 h-screen -z-40 w-80 overflow-hidden border border-border bg-popover shadow-xl">
      <div className="flex h-12 items-center gap-2 border-b border-border px-4 font-semibold">
        <ListMusic size={16} /> Queue{" "}
        {queueItems.length ? `(${queueItems.length})` : ""}
      </div>
      <div ref={scrollRef} className="h-[calc(100%-3rem)] overflow-auto">
        <div
          className="relative"
          style={{ height: `${virtualizer.getTotalSize()}px` }}
        >
          {virtualizer.getVirtualItems().map((item) => {
            const file = queueItems[item.index];
            const isCurrent = item.index === queueInfo.current_index;

            return (
              <SongContextMenu fileId={file.id} key={item.index}>
                <button
                  type="button"
                  key={`${file.id}-${file.occurrence}-${item.index}`}
                  onClick={() => onPlay(item.index)}
                  className={`absolute left-0 flex w-full items-center gap-3 px-3 text-left hover:bg-muted/50 ${isCurrent ? "bg-muted text-primary" : ""}`}
                  style={{
                    height: `${item.size}px`,
                    transform: `translateY(${item.start}px)`,
                  }}
                >
                  <div className="h-8 w-8 shrink-0 rounded bg-muted" />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm">{file.title}</div>
                    <div className="truncate text-xs text-muted-foreground">
                      {file.fileName}
                    </div>
                  </div>
                  <span className="ml-auto text-xs tabular-nums text-muted-foreground">
                    {formatDuration(file.durationMs)}
                  </span>
                </button>
              </SongContextMenu>
            );
          })}
        </div>
      </div>
    </aside>
  );
}
