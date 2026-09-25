import { Link, useRouterState } from "@tanstack/react-router";
import { ChevronDown, Heart, ListMusic, Music4, Settings2 } from "lucide-react";
import { useState } from "react";
import { usePlaylists } from "../hooks/usePlaylists";
import { useStore } from "../hooks/useStore";
import { cn } from "../utils";

type SidebarProps = {
  width: number;
  onWidthChange: (width: number) => void;
};

export default function Sidebar({ width, onWidthChange }: SidebarProps) {
  const router = useRouterState();
  const { openSettings } = useStore();
  const { data: playlists } = usePlaylists();
  const [showPlaylists, setShowPlaylists] = useState(true);

  const links = [
    {
      label: "",
      children: [
        {
          name: "Home",
          path: "/",
        },
        {
          name: "Rewind",
          path: "/rewind",
        },
        {
          name: "Search",
          path: "/search",
        },
        {
          name: "Tag Manager",
          path: "/tagmanager",
        },
      ],
    },
    {
      label: "Library",
      children: [
        {
          name: "Artists",
          path: "/artists",
        },
        {
          name: "Albums",
          path: "/albums",
        },
        {
          name: "Songs",
          path: "/songs",
        },
      ],
    },
  ];

  const isPlaylistSectionActive =
    router.location.pathname.startsWith("/playlists");

  return (
    <aside className="fixed inset-y-0 left-0 z-40  pt-0" style={{ width }}>
      <div className="h-full overflow-hidden rounded-xl bg-popover/95 backdrop-blur">
        <div className="h-12" data-tauri-drag-region={true}></div>
        <div className="h-[calc(100%-7rem)] overflow-auto px-2 pb-4">
          {links.map(({ label, children }) => (
            <div key={label || "primary"} className="py-4">
              <span className="px-2 text-[11px] uppercase text-muted-foreground">
                {label}
              </span>
              <div className="mt-1 flex flex-col gap-0.5 px-1">
                {children.map((child) => {
                  const isActive = router.location.pathname === child.path;
                  return (
                    <Link
                      key={child.path}
                      className={cn(
                        "rounded-lg px-3 py-2 text-[15px] font-medium transition-colors hover:bg-muted/60 hover:text-foreground",
                        isActive && "bg-active text-primary",
                      )}
                      to={child.path}
                    >
                      {child.name}
                    </Link>
                  );
                })}
              </div>
            </div>
          ))}

          <div className="py-4">
            <button
              type="button"
              className="flex w-full items-center justify-between rounded-lg px-2 py-1 text-left"
              onClick={() => setShowPlaylists((open) => !open)}
            >
              <span className="text-[11px] uppercase text-muted-foreground">
                Playlists
              </span>
              <ChevronDown
                size={16}
                className={cn(
                  "text-muted-foreground transition-transform duration-150",
                  showPlaylists && "rotate-180",
                )}
              />
            </button>

            {showPlaylists && (
              <div className="mt-1 flex flex-col gap-0.5 px-1">
                <Link
                  to="/playlists"
                  className={cn(
                    "flex items-center gap-2 rounded-lg px-3 py-2 text-[15px] font-medium transition-colors hover:bg-muted/60 hover:text-foreground",
                    router.location.pathname === "/playlists" &&
                      "bg-active text-primary",
                    isPlaylistSectionActive &&
                      router.location.pathname !== "/playlists" &&
                      "text-foreground",
                  )}
                >
                  <ListMusic size={15} />
                  <span>All Playlists</span>
                </Link>

                <Link
                  to="/favorites"
                  className={cn(
                    "flex items-center gap-2 rounded-lg px-3 py-2 text-[15px] font-medium hover:bg-muted/60",
                    router.location.pathname === "/favorites" &&
                      "bg-active text-primary",
                  )}
                >
                  <Heart size={15} fill="currentColor" />
                  Favorites
                </Link>
                {playlists?.map((playlist) => {
                  const path = `/playlists/${playlist.id}`;
                  const isActive = router.location.pathname === path;
                  return (
                    <Link
                      key={playlist.id}
                      to="/playlists/$playlistId"
                      params={{ playlistId: String(playlist.id) }}
                      className={cn(
                        "flex items-center gap-2 rounded-lg px-3 py-2 text-[15px] font-medium transition-colors hover:bg-muted/60 hover:text-foreground",
                        isActive && "bg-active text-primary",
                        isPlaylistSectionActive &&
                          !isActive &&
                          "text-foreground",
                      )}
                      title={playlist.name}
                    >
                      <Music4
                        size={15}
                        className="shrink-0 text-muted-foreground"
                      />
                      <span className="truncate">{playlist.name}</span>
                    </Link>
                  );
                })}
              </div>
            )}
          </div>
        </div>
        <div className="border-t border-border/60 p-3">
          <button
            type="button"
            onClick={openSettings}
            className="flex w-full items-center gap-2 rounded-xl px-3 py-2.5 text-sm text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
          >
            <Settings2 size={16} />
            Settings<span className="ml-auto text-xs opacity-60">⌘,</span>
          </button>
        </div>
      </div>
      <div
        role="separator"
        aria-label="Resize sidebar"
        aria-orientation="vertical"
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId);
          const startX = event.clientX;
          const startWidth = width;
          const handleMove = (moveEvent: PointerEvent) => {
            onWidthChange(
              Math.min(
                360,
                Math.max(220, startWidth + moveEvent.clientX - startX),
              ),
            );
          };
          const handleUp = () => {
            window.removeEventListener("pointermove", handleMove);
            window.removeEventListener("pointerup", handleUp);
          };
          window.addEventListener("pointermove", handleMove);
          window.addEventListener("pointerup", handleUp, { once: true });
        }}
        className="absolute right-0 top-0 h-full w-1 cursor-col-resize bg-transparent transition-colors hover:bg-primary/50"
      />
    </aside>
  );
}
