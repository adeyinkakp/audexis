import { useEffect, useRef, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useQueryClient } from "@tanstack/react-query";
import toast from "react-hot-toast";
import { useStore } from "../hooks/useStore";
import { useCreatePlaylist } from "../hooks/usePlaylists";
import { PlaylistNameModal } from "../modals/PlaylistNameModal";
import {
  dispatchMenuAction,
  isMac,
  listenMenuAction,
  usePlayerMenuState,
  type AppMenuAction,
} from "../utils/appMenu";
import { updateNativeMenus, type MenuGroup } from "../utils/nativeAppMenu";
import { useSongMenuItems } from "./SongContextMenu";
import { MenuEntries, type MenuItem } from "./ContextMenu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "./Dropdown";

export default function AppMenuBar() {
  const { openSettings, openLibrarySettings, openLogs } = useStore();
  const navigate = useNavigate();
  const client = useQueryClient();
  const createPlaylist = useCreatePlaylist();
  const player = usePlayerMenuState();
  const songItems = useSongMenuItems(player.fileId);
  const [nativeFailed, setNativeFailed] = useState(false);
  const [creating, setCreating] = useState(false);
  const [libraryBusy, setLibraryBusy] = useState(false);
  const busy = useRef(false);

  useEffect(
    () =>
      listenMenuAction((action) => {
        if (action === "settings") openSettings();
      }),
    [openSettings],
  );

  const libraryAction = async (importFolder: boolean) => {
    if (busy.current) return;
    busy.current = true;
    setLibraryBusy(true);
    try {
      if (importFolder) {
        const selected = await invoke<string[]>("import_roots");
        if (!selected.length) return;
        const existing = await invoke<string[]>("get_library_roots");
        await invoke("set_library_roots", {
          folders: [...new Set([...existing, ...selected])],
        });
      } else {
        await invoke("rescan_library");
      }
      await client.invalidateQueries({ queryKey: ["fileWatcherMap"] });
      toast.success(
        importFolder ? "Music folder imported" : "Library scan complete",
      );
    } catch (error) {
      toast.error(String(error));
    } finally {
      busy.current = false;
      setLibraryBusy(false);
    }
  };
  const action = (
    text: string,
    id: AppMenuAction,
    extra: Partial<MenuItem> = {},
  ): MenuItem => ({
    text,
    action: () => dispatchMenuAction(id),
    ...extra,
  });
  const separator = { item: "separator" };
  const groups: MenuGroup[] = [
    {
      text: "File",
      items: [
        {
          text: "New Playlist…",
          disabled: creating,
          action: () => setCreating(true),
        },
        {
          text: libraryBusy ? "Importing / Scanning…" : "Import Folder…",
          disabled: libraryBusy,
          action: () => libraryAction(true),
        },
        separator,
        {
          text: "Library",
          submenu: [
            {
              text: "Manage Music Folders…",
              disabled: libraryBusy,
              action: openLibrarySettings,
            },
            {
              text: "Rescan Library",
              disabled: libraryBusy,
              action: () => libraryAction(false),
            },
            {
              text: "Missing Files",
              action: () => {
                void navigate({ to: "/missing" });
              },
            },
          ],
        },
        ...(!isMac
          ? [separator, { text: "Settings…", action: openSettings }]
          : []),
      ],
    },
    { text: "Song", disabled: player.fileId <= 0, items: songItems() },
    {
      text: "View",
      items: [
        action("Show Queue", "queue", { checked: player.panel === "queue" }),
        action("Show Lyrics", "lyrics", { checked: player.panel === "lyrics" }),
      ],
    },
    {
      text: "Playback",
      items: [
        action("Play", "play", {
          disabled: player.fileId <= 0 || !player.paused,
        }),
        action("Pause", "pause", {
          disabled: player.fileId <= 0 || player.paused,
        }),
        action("Previous Track", "previous", { disabled: player.fileId <= 0 }),
        action("Next Track", "next", { disabled: player.fileId <= 0 }),
        separator,
        action("Shuffle", "shuffle", { checked: player.shuffled }),
        {
          text: "Repeat",
          submenu: [
            action("Off", "repeat-off", { checked: player.repeat === "off" }),
            action("All", "repeat-queue", {
              checked: player.repeat === "queue",
            }),
            action("One", "repeat-track", {
              checked: player.repeat === "track",
            }),
          ],
        },
        separator,
        action("Equalizer…", "equalizer"),
        action("Expand Player", "expanded"),
        action("Open Mini Player", "mini-player"),
      ],
    },
    { text: "Help", items: [{ text: "View Logs…", action: openLogs }] },
  ];

  useEffect(() => {
    if (isMac)
      void updateNativeMenus(groups).catch(() => setNativeFailed(true));
  });

  const buttonClass =
    "rounded-md px-2 py-1.5 text-xs text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-primary data-[state=open]:bg-muted disabled:opacity-40";
  return (
    <>
      {(!isMac || nativeFailed) && (
        <nav
          aria-label="Application menus"
          className="relative z-10 ml-2 flex items-center"
          onKeyDown={(event) => {
            if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
            const buttons = Array.from(
              event.currentTarget.querySelectorAll<HTMLButtonElement>(
                "button:not(:disabled)",
              ),
            );
            const index = buttons.indexOf(
              document.activeElement as HTMLButtonElement,
            );
            if (index < 0) return;
            event.preventDefault();
            buttons[
              (index + (event.key === "ArrowRight" ? 1 : -1) + buttons.length) %
                buttons.length
            ]?.focus();
          }}
        >
          {groups.map((group) => (
            <DropdownMenu key={group.text}>
              <DropdownMenuTrigger asChild>
                <button
                  type="button"
                  disabled={group.disabled}
                  className={buttonClass}
                >
                  {group.text}
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent
                align="start"
                sideOffset={2}
                className="z-13000 min-w-40 max-w-xs text-[13px] leading-4"
              >
                <MenuEntries items={group.items} elevated compact />
              </DropdownMenuContent>
            </DropdownMenu>
          ))}
        </nav>
      )}
      {creating && (
        <PlaylistNameModal
          onClose={() => setCreating(false)}
          onSave={async (name) => {
            const id = await createPlaylist.mutateAsync(name);
            await navigate({
              to: "/playlists/$playlistId",
              params: { playlistId: String(id) },
            });
          }}
        />
      )}
    </>
  );
}
