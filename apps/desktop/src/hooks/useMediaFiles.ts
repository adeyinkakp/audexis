import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import type { FilesResponse } from "./useFileWatcher";

export async function fetchMediaFiles(
  ids: number[],
  signal?: AbortSignal,
): Promise<FilesResponse> {
  const result: FilesResponse = { files: [], metadata: [], next_cursor: null };
  for (let index = 0; index < ids.length; index += 200) {
    signal?.throwIfAborted();
    const batch = await invoke<FilesResponse>("get_media_files", {
      ids: ids.slice(index, index + 200),
    });
    signal?.throwIfAborted();
    result.files.push(...batch.files);
    result.metadata.push(...batch.metadata);
  }
  return result;
}

export function useMediaFiles(ids: number[]) {
  const uniqueIds = [...new Set(ids.filter((id) => id > 0))].sort(
    (a, b) => a - b,
  );
  return useQuery({
    queryKey: ["mediaFiles", uniqueIds],
    queryFn: ({ signal }) => fetchMediaFiles(uniqueIds, signal),
    enabled: uniqueIds.length > 0,
    gcTime: 0,
    staleTime: Infinity,
  });
}
