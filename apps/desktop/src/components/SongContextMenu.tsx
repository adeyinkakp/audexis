import { useRef, type ReactNode } from "react";
import { nextInsertionOrder } from "../utils/songSelection";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import toast from "react-hot-toast";
import { ContextMenuArea, type MenuOptions } from "./ContextMenu";
import { usePlaylists, useAddPlaylistTrack } from "../hooks/usePlaylists";
import { useFavorites } from "../hooks/useFavorites";
import { fetchMediaFiles } from "../hooks/useMediaFiles";
import { useStore } from "../hooks/useStore";
export function useSongMenuItems(
  selection: number | number[],
  playlistId?: number,
  onRemoveFromPlaylist?: () => void | Promise<void>,
) {
  const fileIds = [
    ...new Set(Array.isArray(selection) ? selection : [selection]),
  ].filter((id) => id > 0);
  const fileId = fileIds[0];
  const multiple = fileIds.length > 1;
  const playlists = usePlaylists();
  const add = useAddPlaylistTrack();
  const favorites = useFavorites();
  const navigate = useNavigate();
  const { openTrackInfo } = useStore();
  const running = useRef(false);
  const run = async (action: () => Promise<unknown>) => {
    if (running.current) return;
    running.current = true;
    try {
      await action();
    } catch (error) {
      toast.error(String(error));
    } finally {
      running.current = false;
    }
  };
  return (): MenuOptions => [
    {
      text: "Play Next",
      action: () =>
        run(async () => {
          const queue = await invoke<{ paths: string[] }>("get_queue");
          for (const id of nextInsertionOrder(
            fileIds,
            queue.paths.length === 0,
          ))
            await invoke("enqueue_song", { fileId: id, next: true });
        }),
    },
    {
      text: "Play Last",
      action: () =>
        run(async () => {
          for (const id of fileIds)
            await invoke("enqueue_song", { fileId: id, next: false });
        }),
    },
    { item: "separator" },
    {
      text: "Get Info",
      action: () => {
        openTrackInfo(fileIds);
      },
    },
    {
      text: "Edit Metadata Fields…",
      action: () => openTrackInfo(fileIds, "details"),
    },
    {
      text: "Heart",
      checked: fileIds.every((id) => favorites.data?.includes(id)),
      disabled: !favorites.data,
      action: () =>
        run(async () => {
          const loved = !fileIds.every((id) => favorites.data?.includes(id));
          for (const id of fileIds)
            await favorites.setLovedAsync({ fileId: id, loved });
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
                      for (const id of fileIds)
                        await add.mutateAsync({
                          playlistId: playlist.id,
                          fileId: id,
                        });
                      toast.success(`Added to ${playlist.name}`);
                    }),
                }))
            : [{ text: "No playlists available", disabled: true }],
    },
    { item: "separator" },
    {
      text: "Show Album in Library",
      disabled: multiple,
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
}

export function SongContextMenu({
  fileId,
  fileIds,
  playlistId,
  onRemoveFromPlaylist,
  children,
  asChild = true,
  extraItems,
}: {
  fileId: number;
  fileIds?: number[];
  playlistId?: number;
  onRemoveFromPlaylist?: () => void | Promise<void>;
  children: ReactNode;
  asChild?: boolean;
  extraItems?: MenuOptions | (() => MenuOptions);
}) {
  const items = useSongMenuItems(
    fileIds?.length ? fileIds : fileId,
    playlistId,
    onRemoveFromPlaylist,
  );
  if (fileId <= 0 && !extraItems) return <>{children}</>;
  const menuItems = () => {
    const base = fileId > 0 ? items() : [];
    const extra =
      typeof extraItems === "function" ? extraItems() : (extraItems ?? []);
    return [
      ...base,
      ...(base.length && extra.length ? [{ item: "separator" }] : []),
      ...extra,
    ];
  };
  return (
    <ContextMenuArea asChild={asChild} items={menuItems}>
      {children}
    </ContextMenuArea>
  );
}
