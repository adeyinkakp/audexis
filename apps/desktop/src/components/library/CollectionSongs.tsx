import { SongContextMenu } from "../SongContextMenu";
import { HeartButton } from "../HeartButton";
import { ChevronRight } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import toast from "react-hot-toast";
import {
  useSearchMedia,
  emptySearchFilters,
  type SearchFilters,
} from "../../hooks/useSearchMedia";
import { formatDuration } from "../../utils/duration";
import { Artwork } from "./CollectionGrid";
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
  for (const page of result.data?.pages ?? [])
    for (const tag of page.metadata) {
      const entry = tags.get(tag.file_id) ?? {};
      entry[tag.key] ??= tag.value;
      tags.set(tag.file_id, entry);
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
      ) : result.isError ? (
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
        <div
          className={
            favoritesOnly || filters.exactAlbum !== undefined
              ? "flex flex-col"
              : "grid gap-x-8 lg:grid-cols-2"
          }
        >
          {(preview ? files.slice(0, 5) : files).map((file, index) => {
            const tag = tags.get(file.id) ?? {};
            return (
              <SongContextMenu fileId={file.id} key={file.id}>
                <div
                  onDoubleClick={() => void play(index)}
                  className={`flex min-w-0 items-center gap-3 border-b border-border/60 py-3 px-2 rounded ${currentTrackId === file.id ? "text-primary bg-active" : ""}`}
                >
                  <button
                    aria-label={`Play ${tag.title || file.file_name}`}
                    className="w-14 shrink-0"
                  >
                    <Artwork id={file.id} />
                  </button>
                  <div className="min-w-0 flex-1">
                    <button className="block max-w-full truncate text-left font-medium hover:text-primary">
                      {tag.title || file.file_name}
                    </button>
                    {tag.artist ? (
                      <Link
                        to="/artists"
                        search={{ artist: tag.artist }}
                        className="block truncate text-sm text-muted-foreground hover:text-primary hover:underline"
                      >
                        {tag.artist}
                      </Link>
                    ) : (
                      <span className="text-sm text-muted-foreground">
                        Unknown artist
                      </span>
                    )}
                    {tag.album && (
                      <Link
                        to="/albums"
                        search={{
                          album: tag.album,
                          artist: tag.albumArtist || tag.artist || "",
                        }}
                        className="block truncate text-xs text-muted-foreground hover:text-primary"
                      >
                        {tag.album}
                      </Link>
                    )}
                  </div>
                  <HeartButton fileId={file.id} />
                  <span className="text-xs tabular-nums text-muted-foreground">
                    {formatDuration(file.duration_ms)}
                  </span>
                </div>
              </SongContextMenu>
            );
          })}
        </div>
      )}
      {!preview && result.hasNextPage && (
        <button
          disabled={result.isFetchingNextPage}
          onClick={() => void result.fetchNextPage()}
          className="mt-5 rounded-full border border-border px-5 py-2 text-sm"
        >
          {result.isFetchingNextPage ? "Loading…" : "More songs"}
        </button>
      )}
    </section>
  );
}
