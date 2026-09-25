import type { ReactNode } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import toast from "react-hot-toast";
import { ContextMenuArea, type MenuOptions } from "./ContextMenu";
import { usePlaylists, useAddPlaylistTrack } from "../hooks/usePlaylists";
import { useFavorites } from "../hooks/useFavorites";
import { fetchMediaFiles } from "../hooks/useMediaFiles";
export function SongContextMenu({
  fileId,
  playlistId,
  onRemoveFromPlaylist,
  children,
  asChild = true,
}: {
  fileId: number;
  playlistId?: number;
  onRemoveFromPlaylist?: () => void | Promise<void>;
  children: ReactNode;
  asChild?: boolean;
}) {
  const playlists = usePlaylists();
  const add = useAddPlaylistTrack();
  const favorites = useFavorites();
  const navigate = useNavigate();
  if (fileId <= 0) return <>{children}</>;
  const run = async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (error) {
      toast.error(String(error));
    }
  };
  const items = (): MenuOptions => [
    {
      text: "Play Next",
      action: () => run(() => invoke("enqueue_song", { fileId, next: true })),
    },
    {
      text: "Play Last",
      action: () => run(() => invoke("enqueue_song", { fileId, next: false })),
    },
    { item: "separator" },
    {
      text: "Heart",
      checked: favorites.data?.includes(fileId) ?? false,
      disabled: !favorites.data,
      action: () =>
        favorites.setLoved({
          fileId,
          loved: !favorites.data?.includes(fileId),
        }),
    },
    {
      text: "Add to Playlist",
      submenu: playlists.isPending
        ? [{ text: "Loading playlists…", disabled: true }]
        : playlists.isError
          ? [{ text: "Could not load playlists", disabled: true }]
          : playlists.data?.length
            ? [...playlists.data]
                .sort((a, b) => a.name.localeCompare(b.name))
                .map((playlist) => ({
                  text: playlist.name,
                  disabled: playlist.id === playlistId || add.isPending,
                  action: () =>
                    run(async () => {
                      await add.mutateAsync({
                        playlistId: playlist.id,
                        fileId,
                      });
                      toast.success(`Added to ${playlist.name}`);
                    }),
                }))
            : [{ text: "No playlists available", disabled: true }],
    },
    { item: "separator" },
    {
      text: "Show Album in Library",
      action: () =>
        run(async () => {
          const data = await fetchMediaFiles([fileId]);
          if (!data.files.length)
            throw new Error("This song is no longer in your library");
          const value = (key: string) =>
            data.metadata.find(
              (tag) =>
                tag.file_id === fileId && tag.key === key && tag.value.trim(),
            )?.value;
          const album = value("album") ?? "";
          await navigate({
            to: "/albums",
            search: {
              album,
              artist: album
                ? value("albumArtist") || value("artist") || ""
                : undefined,
            },
          });
        }),
    },
    ...(onRemoveFromPlaylist
      ? [
          { item: "separator" },
          { text: "Remove from Playlist", action: onRemoveFromPlaylist },
        ]
      : []),
  ];
  return (
    <ContextMenuArea asChild={asChild} items={items}>
      {children}
    </ContextMenuArea>
  );
}
