import { useState, useMemo } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Play, Headphones } from "lucide-react";
import toast from "react-hot-toast";
import {
  useRewind,
  rewindIds,
  formatListeningMinutes,
} from "../../hooks/useRewind";
import { useMediaFiles } from "../../hooks/useMediaFiles";
import { Artwork } from "../library/CollectionGrid";
import { HomeShelf } from "../home/HomeShelf";
import { SongContextMenu } from "../SongContextMenu";
const months = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];
export function RewindOverview() {
  const currentYear = new Date().getFullYear();
  const [year, setYear] = useState(currentYear);
  const [month, setMonth] = useState<number | null>(null);
  const result = useRewind(year, month);
  const data = result.data;
  const media = useMediaFiles(rewindIds(data));
  const [playing, setPlaying] = useState(false);
  const files = new Map(media.data?.files.map((file) => [file.id, file]));
  const tags = useMemo(() => {
    const map = new Map<number, Record<string, string>>();
    for (const item of media.data?.metadata ?? []) {
      const tag = map.get(item.file_id) ?? {};
      tag[item.key] ??= item.value;
      map.set(item.file_id, tag);
    }
    return map;
  }, [media.data]);
  const years = [...new Set([currentYear, year, ...(data?.years ?? [])])].sort(
    (a, b) => b - a,
  );
  async function play(id: number) {
    if (playing || !data) return;
    setPlaying(true);
    try {
      const queue = data.songs.flatMap((item) => {
        const file = files.get(item.file_id);
        return file ? [{ id: file.id, path: file.path, occurrence: null }] : [];
      });
      const currentIndex = queue.findIndex((item) => item.id === id);
      if (currentIndex < 0) return;
      await invoke("play_song", {
        playbackInfo: { curr: queue[currentIndex], queue, currentIndex },
      });
    } catch (error) {
      toast.error(String(error));
    } finally {
      setPlaying(false);
    }
  }
  const pending = result.isPending || result.isPlaceholderData;
  const minutes = (us: number) => (
    <p className="mt-2 text-sm font-medium text-primary">
      {formatListeningMinutes(us)}
    </p>
  );
  return (
    <>
      <header className="mb-6 flex flex-wrap items-end justify-between gap-5">
        <div>
          <p className="mb-2 text-xs font-semibold uppercase tracking-widest text-primary">
            Your listening history
          </p>
          <h1 className="text-4xl font-bold tracking-tight">Rewind</h1>
        </div>
        <label className="flex items-center gap-3 text-sm text-muted-foreground">
          Year
          <select
            aria-label="Rewind year"
            value={year}
            onChange={(event) => {
              setYear(Number(event.target.value));
              setMonth(null);
            }}
            className="rounded-full border border-border bg-background px-5 py-3 text-base font-semibold text-foreground"
          >
            {years.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
      </header>
      <nav
        aria-label="Rewind month"
        className="mb-7 flex gap-2 overflow-x-auto pb-3"
      >
        <button
          aria-pressed={month === null}
          onClick={() => setMonth(null)}
          className={`shrink-0 rounded-full px-4 py-2 text-sm ${month === null ? "bg-primary text-primary-foreground" : "bg-muted/40 hover:bg-muted"}`}
        >
          Full Year
        </button>
        {months.map((label, index) => (
          <button
            key={label}
            aria-pressed={month === index + 1}
            onClick={() => setMonth(index + 1)}
            className={`shrink-0 rounded-full px-4 py-2 text-sm ${month === index + 1 ? "bg-primary text-primary-foreground" : "bg-muted/40 hover:bg-muted"}`}
          >
            {label}
          </button>
        ))}
      </nav>
      {result.isError ? (
        <button
          onClick={() => void result.refetch()}
          className="py-8 text-destructive"
        >
          Could not load Rewind. Try again
        </button>
      ) : pending ? (
        <p role="status" className="py-12 text-muted-foreground">
          Loading your listening time…
        </p>
      ) : (
        data && (
          <>
            <section className="mb-5 flex items-center gap-5 rounded-3xl border border-primary/15 bg-primary/5 p-7">
              <div className="rounded-2xl bg-primary/10 p-4 text-primary">
                <Headphones size={32} />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">
                  {month === null
                    ? `Full year · ${year}`
                    : `${months[month - 1]} ${year}`}
                </p>
                <p className="mt-2 text-3xl font-bold tracking-tight">
                  {formatListeningMinutes(data.total_us)}
                </p>
                <p className="mt-2 text-xs text-muted-foreground">
                  Actual listening time.
                </p>
              </div>
            </section>
            {data.total_us === 0 ? (
              <div className="py-10 text-center">
                <h2 className="text-xl font-semibold">
                  Your listening time starts here
                </h2>
                <p className="mt-2 text-sm text-muted-foreground">
                  No listening time recorded for this period.
                </p>
              </div>
            ) : (
              <>
                {data.artists.length > 0 && (
                  <HomeShelf title="Top Artists">
                    {data.artists.map((item, index) => (
                      <Link
                        key={item.name}
                        to="/artists"
                        search={{ artist: item.name }}
                      >
                        <Artwork id={item.file_id} round />
                        <p
                          className="mt-3 truncate font-semibold"
                          title={item.name}
                        >
                          <span className="mr-2 text-muted-foreground">
                            {index + 1}
                          </span>
                          {item.name}
                        </p>
                        {minutes(item.listened_us)}
                      </Link>
                    ))}
                  </HomeShelf>
                )}
                {data.songs.length > 0 && (
                  <HomeShelf title="Top Songs">
                    {data.songs.map((item, index) => {
                      const file = files.get(item.file_id);
                      const tag = tags.get(item.file_id) ?? {};
                      return (
                        <SongContextMenu
                          key={item.file_id}
                          fileId={item.file_id}
                        >
                          <button
                            disabled={playing || !file}
                            onClick={() => void play(item.file_id)}
                            className="group min-w-0 text-left"
                          >
                            <div className="relative">
                              <Artwork id={item.file_id} />
                              <span className="absolute bottom-3 right-3 rounded-full bg-black/60 p-3 text-white opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100">
                                <Play size={18} fill="currentColor" />
                              </span>
                            </div>
                            <p
                              className="mt-3 truncate font-semibold"
                              title={tag.title || file?.file_name}
                            >
                              <span className="mr-2 text-muted-foreground">
                                {index + 1}
                              </span>
                              {tag.title || file?.file_name || "Loading song…"}
                            </p>
                            <p className="mt-1 truncate text-sm text-muted-foreground">
                              {tag.artist || "Unknown artist"}
                            </p>
                            {minutes(item.listened_us)}
                          </button>
                        </SongContextMenu>
                      );
                    })}
                  </HomeShelf>
                )}
                {data.albums.length > 0 && (
                  <HomeShelf title="Top Albums">
                    {data.albums.map((item, index) => (
                      <Link
                        key={JSON.stringify([item.name, item.artist])}
                        to="/albums"
                        search={{ album: item.name, artist: item.artist }}
                      >
                        <Artwork id={item.file_id} />
                        <p
                          className="mt-3 truncate font-semibold"
                          title={item.name}
                        >
                          <span className="mr-2 text-muted-foreground">
                            {index + 1}
                          </span>
                          {item.name}
                        </p>
                        <p className="mt-1 truncate text-sm text-muted-foreground">
                          {item.artist || "Unknown artist"}
                        </p>
                        {minutes(item.listened_us)}
                      </Link>
                    ))}
                  </HomeShelf>
                )}
                {data.playlists.length > 0 && (
                  <HomeShelf
                    title="Top Playlists"
                    subtitle="Time spent playing directly from each playlist."
                  >
                    {data.playlists.map((item, index) => (
                      <Link
                        key={item.playlist_id}
                        to="/playlists/$playlistId"
                        params={{ playlistId: String(item.playlist_id) }}
                      >
                        <Artwork id={item.file_id} />
                        <p
                          className="mt-3 truncate font-semibold"
                          title={item.name}
                        >
                          <span className="mr-2 text-muted-foreground">
                            {index + 1}
                          </span>
                          {item.name}
                        </p>
                        {minutes(item.listened_us)}
                      </Link>
                    ))}
                  </HomeShelf>
                )}
                {media.isError && (
                  <button
                    onClick={() => void media.refetch()}
                    className="py-3 text-sm text-destructive"
                  >
                    Could not load song details. Try again
                  </button>
                )}
              </>
            )}
          </>
        )
      )}
    </>
  );
}
