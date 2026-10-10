import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import toast from "react-hot-toast";
import {
  discoveryIds,
  type useHomeDiscovery,
} from "../../hooks/useHomeDiscovery";
import { useMediaFiles } from "../../hooks/useMediaFiles";
import { useStore } from "../../hooks/useStore";
import { Artwork } from "../library/CollectionGrid";
import { SongItem } from "../songs/SongItem";
import { HomeShelf } from "./HomeShelf";
import { getRelativeTime } from "../../routes/_noneditor";

function formatBytes(bytes: number) {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unit = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  return `${(bytes / 1024 ** unit).toFixed(unit > 1 ? 1 : 0)} ${units[unit]}`;
}

function lastPlayedLabel(timestamp: number | null) {
  if (!timestamp) return "Never played";
  return `Last played ${getRelativeTime(timestamp * 1000)}`;
}

export function HomeDiscovery({
  selection,
}: {
  selection: ReturnType<typeof useHomeDiscovery>;
}) {
  const visibleIds = discoveryIds(selection.data);
  const media = useMediaFiles([...new Set(visibleIds)]);
  const { openSettings } = useStore();
  const [starting, setStarting] = useState(false);
  const files = new Map(media.data?.files.map((file) => [file.id, file]));
  const tags = new Map<number, Record<string, string>>();
  for (const tag of media.data?.metadata ?? []) {
    const entry = tags.get(tag.file_id) ?? {};
    if (tag.value.trim()) entry[tag.key] ??= tag.value;
    tags.set(tag.file_id, entry);
  }
  async function play(ids: number[], index: number) {
    if (starting) return;
    setStarting(true);
    try {
      const queue = ids.flatMap((id) => {
        const file = files.get(id);
        return file ? [{ id, path: file.path, occurrence: null }] : [];
      });
      const currentIndex = queue.findIndex((track) => track.id === ids[index]);
      if (currentIndex < 0) return;
      await invoke("play_song", {
        playbackInfo: { curr: queue[currentIndex], queue, currentIndex },
      });
    } catch (error) {
      toast.error(String(error));
    } finally {
      setStarting(false);
    }
  }
  if (selection.isError || media.isError)
    return (
      <button
        onClick={() => {
          void selection.refetch();
          void media.refetch();
        }}
        className="py-8 text-destructive"
      >
        Could not load your music. Try again
      </button>
    );
  if (selection.isPending || (visibleIds.length > 0 && media.isPending))
    return (
      <p role="status" className="py-10 text-muted-foreground">
        Finding something to listen to…
      </p>
    );
  const data = selection.data!;
  if (!data.recent_ids.length)
    return (
      <section className="rounded-2xl border border-dashed border-border p-10 text-center">
        <h2 className="text-xl font-semibold">
          Start with your own collection
        </h2>
        <p className="mt-2 text-sm text-muted-foreground">
          Add a music folder to discover your music here.
        </p>
        <button
          onClick={() => openSettings("library")}
          className="mt-5 rounded-full bg-primary px-5 py-2.5 text-primary-foreground"
        >
          Add music
        </button>
      </section>
    );
  const listening = data.listening;

  const songItems = (ids: number[]) =>
    ids
      .filter((id) => files.has(id))
      .map((id) => ({ key: String(id), fileId: id }));
  const renderSongs = (
    ids: number[],
    detail: (id: number) => string,
    row = false,
  ) =>
    ids.flatMap((id, index) => {
      const file = files.get(id);
      if (!file) return [];
      const tag = tags.get(id) ?? {};
      return [
        <SongItem
          key={`${id}-${index}`}
          itemKey={String(id)}
          fileId={id}
          title={tag.title || file.file_name}
          artist={tag.artist}
          detail={detail(id)}
          variant={row ? "row" : "card"}
          disabled={starting}
          onPlay={() => void play(ids, index)}
        />,
      ];
    });

  const heavyRotationIds = data.heavy_rotation.map((item) => item.file_id);
  const neglectedIds = data.neglected_songs.map((item) => item.file_id);

  const recent = listening.recent_items
    .filter((item) => files.has(item.file_id))
    .filter((item) => item.kind === "song" || item.kind === "playlist");
  const songIds = recent
    .filter((item) => item.kind === "song")
    .map((item) => item.file_id);
  return (
    <div>
      <HomeShelf
        title="Recently Played"
        subtitle="Your songs"
        songs={songItems(songIds)}
      >
        {recent.map((item) => {
          const tag = tags.get(item.file_id) ?? {};
          const title =
            item.kind === "song"
              ? tag.title || files.get(item.file_id)!.file_name
              : item.name;
          const key = JSON.stringify([
            item.kind,
            item.name,
            item.artist,
            item.playlist_id,
            item.file_id,
          ]);
          const content = (
            <>
              <Artwork id={item.file_id} />
              <p className="mt-3 truncate font-medium" title={title}>
                {title}
              </p>
              <p className="mt-1 truncate text-xs text-primary">
                {item.kind === "song"
                  ? `Song · ${tag.artist || "Unknown artist"}`
                  : item.kind === "album"
                    ? `Album · ${item.artist || "Unknown artist"}`
                    : "Playlist"}
              </p>
            </>
          );
          if (item.kind === "song")
            return (
              <SongItem
                key={key}
                itemKey={String(item.file_id)}
                fileId={item.file_id}
                title={title}
                artist={tag.artist}
                variant="card"
                disabled={starting}
                onPlay={() => void play(songIds, songIds.indexOf(item.file_id))}
              />
            );
          if (item.kind === "album")
            return (
              <Link
                key={key}
                to="/albums"
                search={{ album: item.name, artist: item.artist }}
              >
                {content}
              </Link>
            );
          return (
            <Link
              key={key}
              to="/playlists/$playlistId"
              params={{ playlistId: String(item.playlist_id) }}
            >
              {content}
            </Link>
          );
        })}
      </HomeShelf>
      {!recent.length && (
        <p className="py-8 text-sm text-muted-foreground">
          Play some music and it’ll appear here.
        </p>
      )}

      <HomeShelf
        title="Heavy Rotation"
        songs={songItems(heavyRotationIds)}
        subtitle="Your most-played songs this month"
        row
        action={
          <Link
            to="/rewind"
            className="rounded-full border border-border px-4 py-2 text-sm hover:bg-muted"
          >
            Your Rewind
          </Link>
        }
      >
        {renderSongs(
          heavyRotationIds,
          (id) => {
            const item = data.heavy_rotation.find(
              (song) => song.file_id === id,
            );
            return `${item?.plays ?? 0} ${item?.plays === 1 ? "play" : "plays"}`;
          },
          true,
        )}
      </HomeShelf>
      {!heavyRotationIds.length && (
        <p className="pb-6 text-sm text-muted-foreground">
          Your most-played songs will appear here.
        </p>
      )}
      <HomeShelf
        title="Rediscover"
        songs={songItems(neglectedIds)}
        subtitle="Songs you haven’t heard in a while"
      >
        {renderSongs(neglectedIds, (id) => {
          const item = data.neglected_songs.find((song) => song.file_id === id);
          return lastPlayedLabel(item?.last_played ?? null);
        })}
      </HomeShelf>

      <section className="py-6">
        <header className="mb-5">
          <h2 className="text-2xl font-semibold">Watched Folders</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Indexed music in each library location
          </p>
        </header>
        <div className="grid gap-3 md:grid-cols-2">
          {data.watched_folders.map((folder) => (
            <div
              key={folder.path}
              className="min-w-0 rounded-2xl border border-border/60 bg-card/40 p-4"
            >
              <p className="truncate font-medium" title={folder.path}>
                {folder.path}
              </p>
              <p className="mt-2 text-sm text-muted-foreground">
                {folder.track_count.toLocaleString()} tracks ·{" "}
                {formatBytes(folder.total_size)}
              </p>
              <p className="mt-1 text-xs text-muted-foreground">
                {folder.last_scanned
                  ? `Last scanned ${new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(folder.last_scanned * 1000)}`
                  : "Not scanned yet"}
              </p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
