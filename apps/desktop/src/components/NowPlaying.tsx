import { emitTo, listen } from "@tauri-apps/api/event";
import { AnimatePresence } from "motion/react";
import ExpandedPlayer from "./ExpandedPlayer";
import toast from "react-hot-toast";
import { listenMenuAction, publishPlayerMenuState } from "../utils/appMenu";
import { openMiniPlayer } from "../utils/miniPlayer";
import { SongContextMenu } from "./SongContextMenu";
import { HeartButton } from "./HeartButton";
import { invoke } from "@tauri-apps/api/core";
import { lazy, Suspense, useEffect, useState } from "react";
import {
  ListMusic,
  Maximize2,
  MessageSquareText,
  Pause,
  Play,
  Repeat,
  Shuffle,
  SkipBack,
  SkipForward,
} from "lucide-react";
import SeekBar, { formatSeekTime } from "./SeekBar";
import { cn } from "../utils";
import type { PlaybackPanelView } from "./PlaybackPanel";
const EqualizerModal = lazy(() => import("../modals/EqualizerModal"));
const PlaybackPanel = lazy(() => import("./PlaybackPanel"));
import MarqueeText from "./MarqueeText";
import { useEqualizer } from "../hooks/useEqualizer";
import { useNowPlayingState } from "../hooks/useNowPlayingState";
import { usePlaybackModes, type RepeatMode } from "../hooks/usePlaybackModes";

function ControlButton({
  children,
  title,
  onClick,
  disabled,
  active = false,
  className,
}: {
  children: React.ReactNode;
  title: string;
  onClick: () => void;
  disabled?: boolean;
  active?: boolean;
  className?: string;
}) {
  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:opacity-40",
        active && "text-primary",
        className,
      )}
    >
      {children}
    </button>
  );
}

export default function NowPlaying() {
  const [equalizerOpen, setEqualizerOpen] = useState(false);
  const [expanded, setExpanded] = useState(() =>
    new URLSearchParams(window.location.search).has("expanded-player"),
  );
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen("open-expanded-player", () => setExpanded(true))
      .then((cleanup) => {
        if (disposed) cleanup();
        else {
          unlisten = cleanup;
          if (
            new URLSearchParams(window.location.search).has("expanded-player")
          ) {
            void emitTo("mini-player", "main-player-ready").catch(
              () => undefined,
            );
            const url = new URL(window.location.href);
            url.searchParams.delete("expanded-player");
            window.history.replaceState(window.history.state, "", url);
          }
        }
      })
      .catch(() => toast.error("Could not connect the expanded player."));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  const equalizer = useEqualizer();
  const [panel, setPanel] = useState<PlaybackPanelView | null>(null);
  const [showSeekOverlay, setShowSeekOverlay] = useState(false);
  const {
    currentTrackId,
    song,
    paused,
    position,
    duration,
    setPosition,
    setPaused,
  } = useNowPlayingState();
  const { modes, setModes } = usePlaybackModes();

  const control = async (command: string) => {
    await invoke(command);
    setPaused(command === "pause_playback");
  };

  const showPlayIcon = !song || paused;

  const cycleRepeatMode = async () => {
    const nextMode: RepeatMode =
      modes.repeat_mode === "off"
        ? "queue"
        : modes.repeat_mode === "queue"
          ? "track"
          : "off";

    setModes({ ...modes, repeat_mode: nextMode });
    await invoke("set_repeat_mode", { repeatMode: nextMode });
  };

  const toggleShuffle = async () => {
    setModes({ ...modes, shuffled: !modes.shuffled });
    await invoke("toggle_shuffle");
  };

  useEffect(() => {
    publishPlayerMenuState({
      fileId: currentTrackId,
      paused,
      shuffled: modes.shuffled,
      repeat: modes.repeat_mode,
      panel,
    });
  }, [currentTrackId, paused, modes.shuffled, modes.repeat_mode, panel]);

  useEffect(() =>
    listenMenuAction((action) => {
      const run = (operation: Promise<unknown>) => {
        void operation.catch(() =>
          toast.error("Could not complete the playback action."),
        );
      };
      switch (action) {
        case "equalizer":
          setEqualizerOpen(true);
          break;
        case "mini-player":
          run(openMiniPlayer());
          break;
        case "expanded":
          setExpanded(true);
          break;
        case "queue":
          setPanel((current) => (current === "queue" ? null : "queue"));
          break;
        case "lyrics":
          setPanel((current) => (current === "lyrics" ? null : "lyrics"));
          break;
        case "play":
          if (currentTrackId > 0 && paused) run(control("resume_playback"));
          break;
        case "pause":
          if (currentTrackId > 0 && !paused) run(control("pause_playback"));
          break;
        case "previous":
          if (song) run(control("previous_song"));
          break;
        case "next":
          if (song) run(control("skip_song"));
          break;
        case "shuffle":
          run(toggleShuffle());
          break;
        case "repeat-off":
        case "repeat-queue":
        case "repeat-track": {
          const repeatMode = action.slice(7) as RepeatMode;
          run(invoke("set_repeat_mode", { repeatMode }));
          break;
        }
      }
    }),
  );

  return (
    <div className="fixed bottom-4 left-[calc(var(--sidebar-width,15rem)+1rem)] right-[calc(var(--queue-width,0px)+1.75rem)] z-9999 flex justify-center">
      <div
        className="grid sh-[3.75rem] w-full max-w-5xl grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-1.5 rounded-xl border border-border bg-popover px-1.5 shadow-lg md:px-2"
        onMouseLeave={() => setShowSeekOverlay(false)}
      >
        <div className="flex shrink-0 items-center gap-0.5 md:gap-1">
          <ControlButton
            title="Previous"
            disabled={!song}
            onClick={() => void control("previous_song")}
          >
            <SkipBack size={15} />
          </ControlButton>
          <ControlButton
            title={showPlayIcon ? "Resume" : "Pause"}
            disabled={!song}
            onClick={() =>
              void control(showPlayIcon ? "resume_playback" : "pause_playback")
            }
          >
            {showPlayIcon ? <Play size={15} /> : <Pause size={15} />}
          </ControlButton>
          <ControlButton
            title="Next"
            disabled={!song}
            onClick={() => void control("skip_song")}
          >
            <SkipForward size={15} />
          </ControlButton>
        </div>

        <div className="relative flex min-w-0 w-full flex-col justify-center gap-1 px-1 sm:px-2">
          <SongContextMenu fileId={currentTrackId}>
            <div className="relative flex min-w-0 w-full items-center gap-2 overflow-hidden rounded-lg px-1 py-0.5">
              {song?.artwork_url ? (
                <img
                  src={song.artwork_url}
                  alt=""
                  className="h-8 w-8 shrink-0 rounded-md object-cover sm:h-9 sm:w-9"
                />
              ) : (
                <div className="h-8 w-8 shrink-0 rounded-md bg-muted sm:h-9 sm:w-9" />
              )}
              <div className="min-w-0 flex-1 text-left">
                <MarqueeText className="text-xs font-semibold sm:text-sm">
                  {song?.title || "Nothing is playing"}
                </MarqueeText>
                {song && (
                  <MarqueeText className="text-[11px] text-muted-foreground sm:text-xs">
                    {[song.artist, song.album].filter(Boolean).join(" · ") ||
                      "Unknown artist"}
                  </MarqueeText>
                )}
              </div>

              {currentTrackId != null && (
                <div className="h-full flex items-center">
                  <HeartButton fileId={currentTrackId} />
                </div>
              )}
              <div
                className={cn(
                  "pointer-events-none absolute inset-0 flex items-center justify-between rounded-lg border border-white/10 bg-background/10 px-2 text-[10px] font-medium tabular-nums text-foreground/90 backdrop-blur-[3px] transition-opacity duration-150 sm:px-3 sm:text-[11px]",
                  showSeekOverlay ? "opacity-100" : "opacity-0",
                )}
              >
                <span>{formatSeekTime(position)}</span>
                <span>{formatSeekTime(duration)}</span>
              </div>
            </div>
          </SongContextMenu>

          <SeekBar
            key={currentTrackId}
            compact
            className="w-full"
            position={position}
            duration={duration}
            paused={paused}
            disabled={!song}
            onActivateHover={() => setShowSeekOverlay(true)}
            onSeek={(seconds) => {
              void invoke("seek_playback", {
                milliseconds: Math.round(seconds * 1000),
              });
            }}
          />
        </div>

        <div className="flex shrink-0 items-center gap-0.5 justify-self-end md:gap-1">
          <ControlButton
            title={modes.shuffled ? "Disable shuffle" : "Enable shuffle"}
            disabled={!song}
            active={modes.shuffled}
            onClick={() => void toggleShuffle()}
            className={modes.shuffled ? "bg-muted text-primary" : undefined}
          >
            <Shuffle size={15} />
          </ControlButton>
          <ControlButton
            title={`Repeat: ${modes.repeat_mode}`}
            disabled={!song}
            active={modes.repeat_mode !== "off"}
            onClick={() => void cycleRepeatMode()}
            className={
              modes.repeat_mode === "off" ? undefined : "bg-muted text-primary"
            }
          >
            <div className="relative">
              <Repeat size={15} />
              {modes.repeat_mode === "track" && (
                <span className="pointer-events-none absolute -bottom-1.5 -right-1 text-[8px] font-bold leading-none text-primary">
                  1
                </span>
              )}
            </div>
          </ControlButton>

          <ControlButton
            title="Expand player"
            onClick={() => setExpanded(true)}
          >
            <Maximize2 size={15} />
          </ControlButton>
          <ControlButton
            title="Lyrics"
            active={panel === "lyrics"}
            onClick={() =>
              setPanel((current) => (current === "lyrics" ? null : "lyrics"))
            }
          >
            <MessageSquareText size={15} />
          </ControlButton>

          <ControlButton
            title="Queue"
            active={panel === "queue"}
            onClick={() =>
              setPanel((current) => (current === "queue" ? null : "queue"))
            }
          >
            <ListMusic size={15} />
          </ControlButton>
        </div>
      </div>
      <AnimatePresence>
        {expanded && (
          <ExpandedPlayer
            key="expanded-player"
            onClose={() => setExpanded(false)}
          />
        )}
      </AnimatePresence>
      {equalizerOpen && (
        <Suspense fallback={null}>
          <EqualizerModal
            open
            onClose={() => setEqualizerOpen(false)}
            {...equalizer}
          />
        </Suspense>
      )}
      {panel && (
        <Suspense fallback={null}>
          <PlaybackPanel
            view={panel}
            onViewChange={setPanel}
            onClose={() => setPanel(null)}
            fileId={currentTrackId}
            title={song?.title}
            artist={song?.artist}
            artwork={song?.artwork_url}
            position={position * 1000}
            onSeek={(milliseconds) => {
              void invoke("seek_playback", { milliseconds }).then(() => {
                setPosition(milliseconds / 1000);
              });
            }}
            onPlay={(index) =>
              void invoke("skip_to_index", { index }).then(() =>
                setPaused(false),
              )
            }
          />
        </Suspense>
      )}
    </div>
  );
}
