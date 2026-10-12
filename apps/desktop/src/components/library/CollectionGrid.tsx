import { useArtwork } from "../../hooks/useArtwork";
import { InfiniteScroll } from "../InfiniteScroll";
import { useInfiniteQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { ChevronRight, Disc2, Music2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
export type CollectionKind = "albums" | "artists" | "playlists";
export type Collection = {
  name: string;
  artist: string;
  file_id: number | null;
  playlist_id: number | null;
  track_count: number;
};
type CollectionPage = { items: Collection[]; next_cursor: number | null };
export function Artwork({
  id,
  round = false,
  isAlbum = false,
}: {
  id: number | null;
  round?: boolean;
  isAlbum?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => setVisible(entry.isIntersecting),
      { rootMargin: "100px" },
    );
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      ref={ref}
      className={`flex aspect-square w-full items-center justify-center overflow-hidden bg-linear-to-br from-muted to-primary/20 ${round ? "rounded-full" : "rounded-xl"}`}
    >
      {visible && id !== null ? (
        <VisibleArtwork id={id} isAlbum={isAlbum} />
      ) : isAlbum ? (
        <Disc2 className="h-1/3 w-1/3 text-muted-foreground/50" />
      ) : (
        <Music2 className="h-1/3 w-1/3 text-muted-foreground/50" />
      )}
    </div>
  );
}
function VisibleArtwork({ id, isAlbum }: { id: number; isAlbum: boolean }) {
  const art = useArtwork(id);
  return art.data ? (
    <img src={art.data} alt="" className="h-full w-full object-cover" />
  ) : isAlbum ? (
    <Disc2 className="h-1/3 w-1/3 text-muted-foreground/50" />
  ) : (
    <Music2 className="h-1/3 w-1/3 text-muted-foreground/50" />
  );
}

export function CollectionGrid({
  kind,
  query = "",
  artist,
  enabled = true,
  preview = false,
}: {
  kind: CollectionKind;
  query?: string;
  artist?: string;
  enabled?: boolean;
  preview?: boolean;
}) {
  const [debounced, setDebounced] = useState(query);
  useEffect(() => {
    const timeout = setTimeout(() => setDebounced(query), 100);
    return () => clearTimeout(timeout);
  }, [query]);
  const result = useInfiniteQuery({
    queryKey: ["libraryCollections", kind, debounced, artist],
    initialPageParam: 0,
    queryFn: ({ pageParam }: { pageParam: number }) =>
      invoke<CollectionPage>("browse_library", {
        kind,
        query: debounced,
        artist: artist ?? null,
        offset: pageParam,
      }),
    getNextPageParam: (page) => page.next_cursor ?? undefined,
    enabled: enabled && query === debounced,
    gcTime: 0,
  });
  const allItems = result.data?.pages.flatMap((page) => page.items) ?? [];
  const items = preview ? allItems.slice(0, 5) : allItems;
  return (
    <section className="py-5">
      <h2 className="mb-5 text-2xl font-semibold capitalize">
        {preview ? (
          <Link
            to={
              kind === "playlists"
                ? "/playlists"
                : kind === "albums"
                  ? "/albums"
                  : "/artists"
            }
            search={{ q: query }}
            className="inline-flex items-center gap-2 hover:text-primary"
            aria-label={`See all matching ${kind}`}
          >
            {kind}
            <ChevronRight size={24} />
          </Link>
        ) : (
          kind
        )}
      </h2>
      {result.isError && !result.data ? (
        <button
          className="text-destructive"
          onClick={() => void result.refetch()}
        >
          Could not load {kind}. Try again
        </button>
      ) : result.isPending || query !== debounced ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : !items.length ? (
        <p className="text-sm text-muted-foreground">No matching {kind}.</p>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-5 gap-y-7">
          {items.map((item) => {
            const content = (
              <>
                <Artwork id={item.file_id} round={kind === "artists"} />
                <div
                  className="mt-3 truncate font-medium"
                  title={item.name || "Uncategorized"}
                >
                  {item.name || "Uncategorized"}
                </div>
                <p className="truncate text-sm text-muted-foreground">
                  {kind === "albums" && item.name
                    ? item.artist || "Unknown artist"
                    : `${item.track_count} songs`}
                </p>
              </>
            );
            const key = JSON.stringify([
              item.name,
              item.artist,
              item.playlist_id,
            ]);
            return kind === "playlists" ? (
              <Link
                key={key}
                to="/playlists/$playlistId"
                params={{ playlistId: String(item.playlist_id) }}
                className="min-w-0 rounded-xl focus-visible:outline-primary"
              >
                {content}
              </Link>
            ) : kind === "albums" ? (
              <Link
                key={key}
                to="/albums"
                search={{
                  album: item.name,
                  artist: item.name ? item.artist : artist,
                }}
                className="min-w-0 rounded-xl focus-visible:outline-primary"
              >
                {content}
              </Link>
            ) : (
              <Link
                key={key}
                to="/artists"
                search={{ artist: item.name }}
                className="min-w-0 rounded-xl focus-visible:outline-primary"
              >
                {content}
              </Link>
            );
          })}
        </div>
      )}
      <InfiniteScroll
        enabled={!preview && enabled && query === debounced}
        hasMore={!!result.hasNextPage}
        fetching={result.isFetching}
        error={result.isError}
        itemCount={allItems.length}
        loadMore={result.fetchNextPage}
      />
    </section>
  );
}
