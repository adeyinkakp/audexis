import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
export type ListeningCollection = {
  name: string;
  artist: string;
  file_id: number;
  plays: number;
};
export type Discovery = {
  recent_ids: number[];
  favorite_ids: number[];
  album_ids: number[];
  listening: {
    most_songs: { file_id: number; plays: number }[];
    most_albums: ListeningCollection[];
    most_artists: ListeningCollection[];
    recent_items: {
      kind: "song" | "album" | "playlist";
      name: string;
      artist: string;
      file_id: number;
      playlist_id: number | null;
      played_at: number;
    }[];
  };
};
export function discoveryIds(data?: Discovery) {
  if (!data) return [];
  return [
    ...new Set([
      ...data.recent_ids,
      ...data.favorite_ids,
      ...data.album_ids,
      ...data.listening.recent_items.map((item) => item.file_id),
      ...data.listening.most_songs.map((item) => item.file_id),
      ...data.listening.most_albums.map((item) => item.file_id),
      ...data.listening.most_artists.map((item) => item.file_id),
    ]),
  ];
}
export function useHomeDiscovery(period: string) {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["homeDiscovery", period],
    gcTime: 0,
    staleTime: Infinity,
    refetchOnMount: "always",
    refetchOnWindowFocus: "always",
    queryFn: () => {
      let since: number | null = null;
      let until: number | null = null;
      if (period === "month")
        since = Math.floor(Date.now() / 1000) - 30 * 86400;
      else if (period !== "all") {
        const year = Number(period);
        since = Math.floor(new Date(year, 0, 1).getTime() / 1000);
        until = Math.floor(new Date(year + 1, 0, 1).getTime() / 1000);
      }
      return invoke<Discovery>("get_home_discovery", { since, until });
    },
  });
  useEffect(() => {
    let disposed = false;
    let running = false;
    let dirty = false;
    const cleanups: (() => void)[] = [];
    const queryKey = ["homeDiscovery", period];

    const refresh = async () => {
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
    };
    void Promise.all(
      ["play-count-changed", "listening-history-changed"].map(async (event) => {
        const cleanup = await listen(event, () => {
          void refresh();
        });
        if (disposed) cleanup();
        else cleanups.push(cleanup);
      }),
    )
      .then(() => {
        if (!disposed) void refresh();
      })
      .catch((error) =>
        console.error("Could not listen for history updates", error),
      );
    return () => {
      disposed = true;
      cleanups.forEach((cleanup) => cleanup());
    };
  }, [client, period]);
  return query;
}
