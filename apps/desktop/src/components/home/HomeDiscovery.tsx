import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import toast from "react-hot-toast";
import { useHomeDiscovery } from "../../hooks/useHomeDiscovery";
import { useMediaFiles } from "../../hooks/useMediaFiles";
import { useStore } from "../../hooks/useStore";
import { Artwork } from "../library/CollectionGrid";
import { SongContextMenu } from "../SongContextMenu";
import { HomeShelf } from "./HomeShelf";
export function HomeDiscovery() {
  const selection = useHomeDiscovery("all");
  const visibleIds =
    selection.data?.listening.recent_items.map((item) => item.file_id) ?? [];
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
          onClick={openSettings}
          className="mt-5 rounded-full bg-primary px-5 py-2.5 text-primary-foreground"
        >
          Add music
        </button>
      </section>
    );
  const listening = data.listening;

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
        action={
          <Link
            to="/rewind"
            className="rounded-full border border-border px-4 py-2 text-sm hover:bg-muted"
          >
            Your Rewind
          </Link>
        }
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
              <p className="mt-1 truncate text-xs text-muted-foreground">
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
              <SongContextMenu key={key} fileId={item.file_id}>
                <button
                  disabled={starting}
                  onClick={() =>
                    void play(songIds, songIds.indexOf(item.file_id))
                  }
                  className="min-w-0 text-left"
                >
                  {content}
                </button>
              </SongContextMenu>
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
    </div>
  );
}
