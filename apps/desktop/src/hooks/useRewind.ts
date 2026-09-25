import { useEffect } from "react";
import {
  useQuery,
  useQueryClient,
  keepPreviousData,
} from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export type RewindSong = { file_id: number; listened_us: number };
export type RewindArtist = RewindSong & { name: string };
export type RewindAlbum = RewindArtist & { artist: string };
export type RewindPlaylist = RewindArtist & { playlist_id: number };
export type RewindData = {
  years: number[];
  total_us: number;
  artists: RewindArtist[];
  songs: RewindSong[];
  albums: RewindAlbum[];
  playlists: RewindPlaylist[];
};
export function rewindIds(data?: RewindData) {
  return data
    ? [
        ...new Set(
          [
            ...data.artists,
            ...data.songs,
            ...data.albums,
            ...data.playlists,
          ].map((item) => item.file_id),
        ),
      ]
    : [];
}
export function useRewind(year: number, month: number | null) {
  const client = useQueryClient();
  const result = useQuery({
    queryKey: ["rewind", year, month],
    queryFn: () => invoke<RewindData>("get_rewind", { year, month }),
    gcTime: 0,
    staleTime: Infinity,
    refetchOnMount: "always",
    refetchOnWindowFocus: "always",
    placeholderData: keepPreviousData,
  });
  useEffect(() => {
    let disposed = false;
    let running = false;
    let dirty = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const cleanups: (() => void)[] = [];
    const queryKey = ["rewind", year, month];
    async function refresh() {
      dirty = true;
      if (running) return;
      running = true;
      try {
        while (dirty && !disposed) {
          dirty = false;
          await client.cancelQueries({ queryKey, exact: true });
          if (!disposed)
            await client.invalidateQueries({ queryKey, exact: true });
        }
      } finally {
        running = false;
      }
    }
    const schedule = () => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => void refresh(), 250);
    };
    void Promise.all(
      ["listening-time-changed", "library-changed"].map(async (event) => {
        const cleanup = await listen(event, schedule);
        if (disposed) cleanup();
        else cleanups.push(cleanup);
      }),
    )
      .then(() => {
        if (!disposed) void refresh();
      })
      .catch(console.error);
    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      cleanups.forEach((cleanup) => cleanup());
    };
  }, [client, year, month]);
  return result;
}
export function formatListeningMinutes(microseconds: number) {
  if (microseconds > 0 && microseconds < 60_000_000)
    return "Less than 1 minute";
  const minutes = Math.floor(microseconds / 60_000_000);
  return `${minutes.toLocaleString()} ${minutes === 1 ? "minute" : "minutes"}`;
}
