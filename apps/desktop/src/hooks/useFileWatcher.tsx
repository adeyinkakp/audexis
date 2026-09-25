import { useEffect, useState } from "react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export type DatabaseMediaFile = {
  id: number;
  path: string;
  file_name: string;
  last_validated: number;
  status: string;
  duration_ms: number | null;
  format: string | null;
  modified_at: number;
  size: number;
};

export type DatabaseMediaMetadata = {
  file_id: number;
  key: string;
  value: string;
  ord: number;
};

export type FilesResponse = {
  files: DatabaseMediaFile[];
  metadata: DatabaseMediaMetadata[];
  next_cursor: number | null;
};

export interface NormalizedMediaFiles {
  filesById: Record<number, DatabaseMediaFile>;
  metadataByFileId: Record<number, DatabaseMediaMetadata[]>;
  allIds: number[];
  nextCursor: number | null;
}

export function useFileWatcher(search = "") {
  const [debounced, setDebounced] = useState(search.trim());
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(search.trim()), 250);
    return () => clearTimeout(timer);
  }, [search]);
  const queryKey = debounced
    ? ["searchMedia", "songs", debounced]
    : ["fileWatcherMap"];

  const query = useInfiniteQuery<
    FilesResponse,
    Error,
    NormalizedMediaFiles,
    string[],
    number
  >({
    queryKey,
    initialPageParam: 0,
    queryFn: async ({ pageParam }) => {
      if (debounced)
        return invoke<FilesResponse>("search_media", {
          input: { query: debounced, offset: pageParam },
        });
      const response = await invoke<FilesResponse>("get_media_page", {
        cursor: pageParam || null,
        limit: 100,
      });

      return response;
    },
    getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
    select: (result) => {
      const filesById: Record<number, DatabaseMediaFile> = {};
      const metadataByFileId: Record<number, DatabaseMediaMetadata[]> = {};
      const allIds: number[] = [];

      for (const page of result.pages) {
        for (const file of page.files) {
          filesById[file.id] = file;
          allIds.push(file.id);
        }

        for (const metadata of page.metadata) {
          (metadataByFileId[metadata.file_id] ??= []).push(metadata);
        }
      }

      return {
        filesById,
        metadataByFileId,
        allIds,
        nextCursor: result.pages[result.pages.length - 1]?.next_cursor ?? null,
      };
    },
    gcTime: 0,
    staleTime: Infinity,
    refetchOnMount: "always",
  });

  return query;
}
