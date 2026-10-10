import { useEffect, useState } from "react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { FilesResponse } from "./useFileWatcher";
export type SearchFilters = {
  title: string;
  artist: string;
  album: string;
  fileName: string;
  genre: string;
  format: string;
  folder: string;
  minMinutes: string;
  maxMinutes: string;
  exactAlbum?: string;
  exactArtist?: string;
};
export const emptySearchFilters: SearchFilters = {
  title: "",
  artist: "",
  album: "",
  fileName: "",
  genre: "",
  format: "",
  folder: "",
  minMinutes: "",
  maxMinutes: "",
};
export function useSearchMedia(
  query: string,
  filters: SearchFilters,
  favoritesOnly = false,
) {
  const min = filters.minMinutes.trim()
    ? Number(filters.minMinutes)
    : undefined;
  const max = filters.maxMinutes.trim()
    ? Number(filters.maxMinutes)
    : undefined;
  const invalidDuration =
    [min, max].some(
      (value) =>
        value !== undefined &&
        (!Number.isFinite(value) || value < 0 || value > 1000000),
    ) ||
    (min !== undefined && max !== undefined && min > max);
  const input = JSON.stringify({
    query: query.trim(),
    favoritesOnly,
    ...filters,
    minDurationMs:
      min === undefined || invalidDuration ? null : Math.round(min * 60000),
    maxDurationMs:
      max === undefined || invalidDuration ? null : Math.round(max * 60000),
  });
  const [debounced, setDebounced] = useState(input);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(input), 250);
    return () => clearTimeout(timer);
  }, [input]);
  const hasCriteria =
    favoritesOnly ||
    filters.exactAlbum !== undefined ||
    filters.exactArtist !== undefined ||
    !!query.trim() ||
    Object.values(filters).some((value) => value?.trim());
  const results = useInfiniteQuery({
    queryKey: ["searchMedia", debounced],
    initialPageParam: 0,
    queryFn: async ({ pageParam, signal }) => {
      signal.throwIfAborted();
      const result = await invoke<FilesResponse>("search_media", {
        input: { ...JSON.parse(debounced), offset: pageParam },
      });
      signal.throwIfAborted();
      return result;
    },
    getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
    enabled: hasCriteria && !invalidDuration && input === debounced,
    gcTime: 0,
    staleTime: Infinity,
  });
  return {
    ...results,
    hasCriteria,
    invalidDuration,
    isDebouncing: input !== debounced,
  };
}
