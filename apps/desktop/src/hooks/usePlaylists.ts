import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

export type PlaylistSummary = {
  id: number;
  name: string;
  track_count: number;
  artwork_file_id: number | null;
};

export type PlaylistTrack = {
  id: number;
  path: string;
  file_name: string;
  duration_ms: number | null;
  ord: number;
};

export type PlaylistDetail = {
  id: number;
  name: string;
  tracks: PlaylistTrack[];
};

export function usePlaylists() {
  return useQuery({
    queryKey: ["playlists"],
    queryFn: () => invoke<PlaylistSummary[]>("get_playlists"),
  });
}

export function usePlaylist(playlistId: number | null) {
  return useQuery({
    queryKey: ["playlist", playlistId],
    gcTime: 0,
    queryFn: () => invoke<PlaylistDetail>("get_playlist", { playlistId }),
    enabled: playlistId !== null,
  });
}

export function useCreatePlaylist() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (name: string) =>
      invoke<number>("create_playlist", { input: { name } }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["playlists"] });
      void queryClient.invalidateQueries({ queryKey: ["homeDiscovery"] });
      void queryClient.invalidateQueries({
        queryKey: ["libraryCollections", "playlists"],
      });
    },
  });
}

export function useRenamePlaylist() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, name }: { id: number; name: string }) =>
      invoke("rename_playlist", { input: { id, name } }),
    onError: () => {},
    onSuccess: (_data, variables) => {
      void queryClient.invalidateQueries({ queryKey: ["playlists"] });
      void queryClient.invalidateQueries({ queryKey: ["homeDiscovery"] });
      void queryClient.invalidateQueries({
        queryKey: ["libraryCollections", "playlists"],
      });
      void queryClient.invalidateQueries({
        queryKey: ["playlist", variables.id],
      });
    },
  });
}

export function useDeletePlaylist() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (playlistId: number) =>
      invoke("delete_playlist", { playlistId }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["playlists"] });
      void queryClient.invalidateQueries({ queryKey: ["homeDiscovery"] });
      void queryClient.invalidateQueries({
        queryKey: ["libraryCollections", "playlists"],
      });
    },
  });
}

export function useAddPlaylistTrack() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      playlistId,
      fileId,
    }: {
      playlistId: number;
      fileId: number;
    }) =>
      invoke("add_playlist_track", {
        input: { playlist_id: playlistId, file_id: fileId },
      }),
    onSuccess: (_data, variables) => {
      void queryClient.invalidateQueries({
        queryKey: ["playlist", variables.playlistId],
      });
      void queryClient.invalidateQueries({ queryKey: ["playlists"] });
      void queryClient.invalidateQueries({ queryKey: ["homeDiscovery"] });
      void queryClient.invalidateQueries({
        queryKey: ["libraryCollections", "playlists"],
      });
    },
  });
}

export function useRemovePlaylistTrack() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ playlistId, ord }: { playlistId: number; ord: number }) =>
      invoke("remove_playlist_track", {
        input: { playlist_id: playlistId, ord },
      }),
    onSuccess: (_data, variables) => {
      void queryClient.invalidateQueries({
        queryKey: ["playlist", variables.playlistId],
      });
      void queryClient.invalidateQueries({ queryKey: ["playlists"] });
      void queryClient.invalidateQueries({ queryKey: ["homeDiscovery"] });
      void queryClient.invalidateQueries({
        queryKey: ["libraryCollections", "playlists"],
      });
    },
  });
}

export function useReorderPlaylistTrack() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      playlistId,
      fromOrd,
      toOrd,
    }: {
      playlistId: number;
      fromOrd: number;
      toOrd: number;
    }) =>
      invoke("reorder_playlist_track", {
        input: {
          playlist_id: playlistId,
          from_ord: fromOrd,
          to_ord: toOrd,
        },
      }),
    onSuccess: (_data, variables) => {
      return queryClient.invalidateQueries({
        queryKey: ["playlist", variables.playlistId],
      });
    },
  });
}
