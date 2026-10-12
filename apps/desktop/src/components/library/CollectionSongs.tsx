import { InfiniteScroll } from "../InfiniteScroll";
import { Fragment } from "react";
import { SongCollection } from "../songs/SongCollection";
import { SongItem } from "../songs/SongItem";
import { ChevronRight, Disc2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import toast from "react-hot-toast";
import {
  useSearchMedia,
  emptySearchFilters,
  type SearchFilters,
} from "../../hooks/useSearchMedia";
import { formatDuration } from "../../utils/duration";
import { useNowPlayingState } from "../../hooks/useNowPlayingState";
export function CollectionSongs({
  query = "",
  preview = false,
  favoritesOnly = false,
  filters = emptySearchFilters,
}: {
  query?: string;
  preview?: boolean;
  favoritesOnly?: boolean;
  filters?: SearchFilters;
}) {
  const result = useSearchMedia(query, filters, favoritesOnly);
  const { currentTrackId } = useNowPlayingState();
  const files = [
    ...new Map(
      (result.data?.pages ?? [])
        .flatMap((page) => page.files)
        .map((file) => [file.id, file]),
    ).values(),
  ];
  const tags = new Map<number, Record<string, string>>();
  const discs = new Map<number, number>();
  for (const page of result.data?.pages ?? [])
    for (const tag of page.metadata) {
      const entry = tags.get(tag.file_id) ?? {};
      entry[tag.key] ??= tag.value;
      tags.set(tag.file_id, entry);
      if (tag.key === "discnumber" || tag.key === "discNumber") {
        const disc = Number.parseInt(tag.value, 10);
        if (disc > 0 && !discs.has(tag.file_id)) discs.set(tag.file_id, disc);
      }
    }
  async function play(index: number) {
    const queue = files.map((file) => ({
      id: file.id,
      path: file.path,
      occurrence: null,
    }));
    try {
      await invoke("play_song", {
        playbackInfo: { curr: queue[index], queue, currentIndex: index },
      });
    } catch (error) {
      toast.error(String(error));
    }
  }
  return (
    <section className="py-5">
      <h2 className="mb-4 text-2xl font-semibold">
        {preview ? (
          <Link
            to="/songs"
            search={{ q: query }}
            aria-label="See all matching songs"
            className="inline-flex items-center gap-2 hover:text-primary"
          >
            Songs
            <ChevronRight size={24} />
          </Link>
        ) : (
          "Songs"
        )}
      </h2>
      {result.invalidDuration ? (
        <p role="alert">Enter a valid duration range.</p>
      ) : result.isError && !result.data ? (
        <button
          className="text-destructive"
          onClick={() => void result.refetch()}
        >
          Could not search songs. Try again
        </button>
      ) : result.isPending || result.isDebouncing ? (
        <p className="text-muted-foreground">Searching…</p>
      ) : !files.length ? (
        <p className="text-sm text-muted-foreground">No matching songs.</p>
      ) : (
        <SongCollection
          key={JSON.stringify([query, filters, favoritesOnly])}
          items={(preview ? files.slice(0, 5) : files).map((file) => ({
            key: String(file.id),
            fileId: file.id,
          }))}
          hasMore={!preview && result.hasNextPage}
        >
          <div
            className={
              favoritesOnly || filters.exactAlbum !== undefined
                ? "flex flex-col"
                : "grid gap-x-8 lg:grid-cols-2"
            }
          >
            {(preview ? files.slice(0, 5) : files).map((file, index) => {
              const tag = tags.get(file.id) ?? {};
              const disc = discs.get(file.id);
              const trackNumber = Number.parseInt(tag.trackNumber ?? "", 10);
              const startsDisc =
                filters.exactAlbum !== undefined &&
                disc !== undefined &&
                (index === 0 || discs.get(files[index - 1].id) !== disc);
              return (
                <Fragment key={file.id}>
                  {startsDisc && (
                    <h3 className="mb-2 mt-6 flex items-center gap-2 border-b border-border/60 px-3 pb-3 text-sm font-semibold text-muted-foreground first:mt-0">
                      <Disc2 size={17} aria-hidden="true" />
                      Disc {disc}
                    </h3>
                  )}
                  <SongItem
                    itemKey={String(file.id)}
                    fileId={file.id}
                    title={tag.title || file.file_name}
                    artist={tag.artist}
                    trackNumber={
                      filters.exactAlbum !== undefined && trackNumber > 0
                        ? trackNumber
                        : undefined
                    }
                    detail={formatDuration(file.duration_ms)}
                    current={currentTrackId === file.id}
                    favorite
                    onPlay={() => void play(index)}
                  />
                </Fragment>
              );
            })}
          </div>
        </SongCollection>
      )}
      <InfiniteScroll
        enabled={!preview && !result.isDebouncing && !result.invalidDuration}
        hasMore={!!result.hasNextPage}
        fetching={result.isFetching}
        error={result.isError}
        itemCount={files.length}
        loadMore={result.fetchNextPage}
      />
    </section>
  );
}
