import { discoveryIds, type Discovery } from "./useHomeDiscovery";
import { useEffect } from "react";
import { useQueryClient, type InfiniteData } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { fetchMediaFiles } from "./useMediaFiles";
import type { FilesResponse } from "./useFileWatcher";
import type { PlaylistDetail } from "./usePlaylists";

export function useLibraryEvents() {
  const client = useQueryClient();
  useEffect(() => {
    let disposed = false;
    let running = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let unlisten: (() => void) | undefined;
    const pending = new Set<number>();

    const flush = async () => {
      timer = undefined;
      if (running || disposed) return;
      running = true;
      try {
        while (pending.size && !disposed) {
          const changed = new Set(pending);
          pending.clear();

          await Promise.allSettled(
            client
              .getQueryCache()
              .findAll({ type: "active" })
              .filter((query) =>
                [
                  "fileWatcherMap",
                  "mediaFiles",
                  "playlist",
                  "searchMedia",
                  "libraryCollections",
                  "homeDiscovery",
                ].includes(String(query.queryKey[0])),
              )
              .map((query) => query.promise),
          );
          if (disposed) break;
          const loaded = new Set<number>();
          const queries = client.getQueryCache().findAll({ type: "active" });
          for (const query of queries) {
            const data = query.state.data;
            if (
              query.queryKey[0] === "fileWatcherMap" ||
              query.queryKey[0] === "searchMedia"
            ) {
              (data as InfiniteData<FilesResponse> | undefined)?.pages.forEach(
                (page) => page.files.forEach((file) => loaded.add(file.id)),
              );
            } else if (query.queryKey[0] === "libraryCollections") {
              (
                data as
                  | InfiniteData<{ items: { file_id: number | null }[] }>
                  | undefined
              )?.pages.forEach((page) =>
                page.items.forEach((item) => {
                  if (item.file_id !== null) loaded.add(item.file_id);
                }),
              );
            } else if (query.queryKey[0] === "homeDiscovery") {
              discoveryIds(data as Discovery | undefined).forEach((id) =>
                loaded.add(id),
              );
            } else if (query.queryKey[0] === "mediaFiles") {
              (query.queryKey[1] as number[]).forEach((id) => loaded.add(id));
            } else if (query.queryKey[0] === "playlist") {
              (data as PlaylistDetail | undefined)?.tracks.forEach((track) =>
                loaded.add(track.id),
              );
            }
          }
          const ids = [...changed].filter((id) => loaded.has(id));
          if (!ids.length) continue;
          const fresh = await fetchMediaFiles(ids);
          if (disposed) break;
          const affected = new Set(ids);
          const files = new Map(fresh.files.map((file) => [file.id, file]));
          const patch = (old: FilesResponse): FilesResponse => {
            if (!old.files.some((file) => affected.has(file.id))) return old;
            const present = new Set(old.files.map((file) => file.id));
            return {
              ...old,
              files: old.files.flatMap((file) =>
                affected.has(file.id)
                  ? files.get(file.id)
                    ? [files.get(file.id)!]
                    : []
                  : [file],
              ),
              metadata: [
                ...old.metadata.filter((item) => !affected.has(item.file_id)),
                ...fresh.metadata.filter((item) => present.has(item.file_id)),
              ],
            };
          };
          for (const query of client
            .getQueryCache()
            .findAll({ type: "active" })) {
            if (query.queryKey[0] === "homeDiscovery") {
              if (
                discoveryIds(query.state.data as Discovery | undefined).some(
                  (id) => affected.has(id),
                )
              )
                void client.invalidateQueries({
                  queryKey: query.queryKey,
                  exact: true,
                });
            } else if (query.queryKey[0] === "libraryArtwork") {
              if (affected.has(query.queryKey[1] as number))
                void client.invalidateQueries({
                  queryKey: query.queryKey,
                  exact: true,
                });
            } else if (query.queryKey[0] === "libraryCollections") {
              const data = query.state.data as
                | InfiniteData<{ items: { file_id: number | null }[] }>
                | undefined;
              if (
                data?.pages.some((page) =>
                  page.items.some(
                    (item) =>
                      item.file_id !== null && affected.has(item.file_id),
                  ),
                )
              )
                void client.invalidateQueries({
                  queryKey: query.queryKey,
                  exact: true,
                });
            } else if (query.queryKey[0] === "searchMedia") {
              const data = query.state.data as
                InfiniteData<FilesResponse> | undefined;
              if (
                data?.pages.some((page) =>
                  page.files.some((file) => affected.has(file.id)),
                )
              ) {
                void client.invalidateQueries({
                  queryKey: query.queryKey,
                  exact: true,
                });
              }
            } else if (query.queryKey[0] === "fileWatcherMap") {
              client.setQueryData<InfiniteData<FilesResponse>>(
                query.queryKey,
                (old) => old && { ...old, pages: old.pages.map(patch) },
              );
            } else if (query.queryKey[0] === "mediaFiles") {
              const requested = query.queryKey[1] as number[];
              if (!requested.some((id) => affected.has(id))) continue;

              if (query.state.fetchStatus === "fetching") {
                void client
                  .cancelQueries({ queryKey: query.queryKey, exact: true })
                  .then(() =>
                    client.invalidateQueries({
                      queryKey: query.queryKey,
                      exact: true,
                    }),
                  );
              } else {
                client.setQueryData<FilesResponse>(
                  query.queryKey,
                  (old) => old && patch(old),
                );
              }
            } else if (query.queryKey[0] === "playlist") {
              client.setQueryData<PlaylistDetail>(
                query.queryKey,
                (old) =>
                  old && {
                    ...old,
                    tracks: old.tracks.flatMap((track) => {
                      if (!affected.has(track.id)) return [track];
                      const file = files.get(track.id);
                      return file
                        ? [
                            {
                              ...track,
                              path: file.path,
                              file_name: file.file_name,
                              duration_ms: file.duration_ms,
                            },
                          ]
                        : [];
                    }),
                  },
              );
            }
          }
        }
      } catch (error) {
        console.error("Could not refresh loaded file metadata", error);
      } finally {
        running = false;
      }
    };

    void listen<{ file_ids: number[] }>("library-changed", ({ payload }) => {
      payload.file_ids.forEach((id) => pending.add(id));
      if (!timer && !running) timer = setTimeout(() => void flush(), 100);
    }).then((cleanup) => {
      if (disposed) cleanup();
      else unlisten = cleanup;
    });
    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      pending.clear();
      unlisten?.();
    };
  }, [client]);
}
