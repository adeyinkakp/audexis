import type { QueryClient, QueryKey } from "@tanstack/react-query";

const libraryQueries = new Set([
  "fileWatcherMap",
  "searchMedia",
  "libraryCollections",
  "homeDiscovery",
  "playlist",
  "playlists",
  "favoriteIds",
  "missingFiles",
  "rewind",
]);

function affectedQuery(key: QueryKey, ids: ReadonlySet<number>) {
  if (libraryQueries.has(String(key[0]))) return true;
  if (key[0] === "libraryArtwork") return ids.has(key[1] as number);
  if (key[0] === "mediaFiles" || key[0] === "artworkDetails" || key[0] === "lyrics") {
    return Array.isArray(key[1]) && key[1].some((id: number) => ids.has(id));
  }
  return false;
}

export async function refreshLibraryQueries(
  client: QueryClient,
  ids: ReadonlySet<number>,
  disposed: () => boolean = () => false,
) {
  if (!ids.size || disposed()) return;
  const filters = {
    predicate: (query: { queryKey: QueryKey }) =>
      affectedQuery(query.queryKey, ids),
  };

  await client.cancelQueries(filters);
  if (disposed()) return;
  await client.invalidateQueries(
    { ...filters, refetchType: "active" },
    { throwOnError: true },
  );
}

type Subscribe = (onChange: (ids: number[]) => void) => Promise<() => void>;

export function subscribeLibraryChanges(
  client: QueryClient,
  subscribe: Subscribe,
  onError: (error: unknown) => void,
) {
  let disposed = false;
  let running = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let unlisten: (() => void) | undefined;
  const pending = new Set<number>();

  function schedule() {
    if (!disposed && !running && timer === undefined && pending.size) {
      timer = setTimeout(() => void flush(), 100);
    }
  }

  async function flush() {
    timer = undefined;
    if (disposed) return;
    const ids = new Set(pending);
    pending.clear();
    running = true;
    try {
      await refreshLibraryQueries(client, ids, () => disposed);
    } catch (error) {
      if (!disposed) onError(error);
    } finally {
      running = false;

      schedule();
    }
  }

  void subscribe((ids) => {
    if (disposed) return;
    ids.forEach((id) => pending.add(id));
    schedule();
  })
    .then((cleanup) => {
      if (disposed) cleanup();
      else unlisten = cleanup;
    })
    .catch((error) => {
      if (!disposed) onError(error);
    });

  return () => {
    if (disposed) return;
    disposed = true;
    if (timer !== undefined) clearTimeout(timer);
    pending.clear();
    unlisten?.();
  };
}
