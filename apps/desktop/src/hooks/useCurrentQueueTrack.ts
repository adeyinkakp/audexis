import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";

type QueueInfo = {
  paths: string[];
  current_index: number;
  occurrences: (number | null)[];
};

export function useCurrentQueueTrack() {
  const [queueInfo, setQueueInfo] = useState<QueueInfo>({
    paths: [],
    occurrences: [],
    current_index: 0,
  });

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    void invoke<QueueInfo>("get_queue").then((result) => setQueueInfo(result));
    void listen<QueueInfo>("queue-changed", (event) => {
      setQueueInfo(event.payload);
    }).then((unlisten) => {
      cleanup = unlisten;
    });

    return () => cleanup?.();
  }, []);

  const currentQueueTrack = useMemo(() => {
    if (
      queueInfo.current_index < 0 ||
      queueInfo.current_index >= queueInfo.paths.length
    ) {
      return null;
    }

    const currentPath = queueInfo.paths[queueInfo.current_index];
    let occurrence = 0;

    for (let index = 0; index < queueInfo.current_index; index += 1) {
      if (queueInfo.paths[index] === currentPath) {
        occurrence += 1;
      }
    }

    return {
      path: currentPath,
      occurrence: queueInfo.occurrences[queueInfo.current_index] ?? occurrence,
    };
  }, [queueInfo]);

  return currentQueueTrack;
}
