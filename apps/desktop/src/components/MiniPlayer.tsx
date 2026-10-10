import { invoke } from "@tauri-apps/api/core";
import { showMainPlayerWindow } from "../utils/mainPlayerWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Shuffle,
  Repeat,
  Repeat1,
  ListMusic,
  MessageSquareText,
  Maximize2,
  ChevronDown,
  Music2,
  Pause,
  Pin,
  Play,
  SkipBack,
  SkipForward,
  X,
} from "lucide-react";
import { lazy, Suspense, useEffect, useRef, useState } from "react";
import toast from "react-hot-toast";
import { usePlaybackModes, type RepeatMode } from "../hooks/usePlaybackModes";
import { useNowPlayingState } from "../hooks/useNowPlayingState";
import { logError } from "../utils/logger";
import type { PlaybackPanelView } from "./PlaybackPanel";
const PlaybackPanel = lazy(() => import("./PlaybackPanel"));
import ArtworkBackdrop from "./ArtworkBackdrop";
import SeekBar from "./SeekBar";
import { cn } from "../utils";

export default function MiniPlayer({
  expanded = false,
  onCollapse,
  reducedMotion = false,
}: {
  expanded?: boolean;
  onCollapse?: () => void;
  reducedMotion?: boolean;
}) {
  const fullscreen = expanded;
  const { modes } = usePlaybackModes();
  const nextRepeatMode: RepeatMode =
    modes.repeat_mode === "off"
      ? "queue"
      : modes.repeat_mode === "queue"
        ? "track"
        : "off";
  const repeatLabels = { off: "Off", queue: "All", track: "One" };
  const repeatLabel = `Repeat: ${repeatLabels[modes.repeat_mode]}. Switch to ${repeatLabels[nextRepeatMode]}`;
  const { song, paused, position, duration } = useNowPlayingState();
  const [panel, setPanel] = useState<PlaybackPanelView | null>(null);
  const lyricsToggle = useRef<HTMLButtonElement>(null);
  const queueToggle = useRef<HTMLButtonElement>(null);
  function closePanel() {
    (panel === "lyrics" ? lyricsToggle : queueToggle).current?.focus();
    setPanel(null);
  }
  const [pinned, setPinned] = useState(false);
  const [changingWindow, setChangingWindow] = useState(false);
  const [failedArtwork, setFailedArtwork] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function run(action: () => Promise<unknown>) {
    try {
      await action();
    } catch (error) {
      logError("Mini player action failed", error);
      toast.error("Could not complete that action. Please try again.");
    }
  }

  useEffect(() => {
    if (!expanded)
      void getCurrentWindow()
        .show()
        .catch((error) => logError("Show mini player", error));
  }, [expanded]);

  async function expandInMainWindow() {
    setChangingWindow(true);
    try {
      await run(async () => {
        await showMainPlayerWindow();
        await getCurrentWindow().close();
      });
    } finally {
      setChangingWindow(false);
    }
  }

  async function control(command: string, args?: Record<string, unknown>) {
    setBusy(true);
    try {
      await run(() => invoke(command, args));
    } finally {
      setBusy(false);
    }
  }
  function seek(seconds: number) {
    void run(() =>
      invoke("seek_playback", { milliseconds: Math.round(seconds * 1000) }),
    );
  }
  const button =
    "flex size-8 items-center justify-center rounded-full text-foreground/70 transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-primary disabled:opacity-30";
  const artwork = song?.artwork_url;

  return (
    <main
      onKeyDown={(event) => {
        if (event.key === "Escape" && panel) {
          event.preventDefault();
          event.stopPropagation();
          closePanel();
        }
      }}
      aria-label={fullscreen ? "Full-screen player" : "Mini player"}
      className="relative isolate flex h-full flex-col overflow-hidden border border-border bg-background text-foreground select-none"
    >
      <ArtworkBackdrop artwork={artwork} />
      <header
        data-tauri-drag-region
        className={
          fullscreen
            ? "flex h-16 shrink-0 items-center gap-2 px-6"
            : "flex h-11 shrink-0 items-center gap-1 px-3"
        }
      >
        <span
          data-tauri-drag-region
          className={cn(
            "flex-1 text-[10px] font-semibold tracking-[0.18em] text-foreground/50",
            expanded && "ml-24",
          )}
        >
          AUDEXIS
        </span>
        {expanded && (
          <div className="bg-muted/60 py-1 flex px-2 rounded-xl">
            <button
              ref={lyricsToggle}
              className={button}
              aria-label="Lyrics"
              title="Lyrics"
              aria-pressed={panel === "lyrics"}
              aria-expanded={panel === "lyrics"}
              aria-controls="expanded-playback-panel"
              onClick={() =>
                setPanel((current) => (current === "lyrics" ? null : "lyrics"))
              }
            >
              <MessageSquareText
                size={18}
                className={panel === "lyrics" ? "text-primary" : undefined}
              />
            </button>
            <button
              ref={queueToggle}
              className={button}
              aria-label="Queue"
              title="Queue"
              aria-pressed={panel === "queue"}
              aria-expanded={panel === "queue"}
              aria-controls="expanded-playback-panel"
              onClick={() =>
                setPanel((current) => (current === "queue" ? null : "queue"))
              }
            >
              <ListMusic
                size={18}
                className={panel === "queue" ? "text-primary" : undefined}
              />
            </button>
          </div>
        )}
        {!expanded && (
          <button
            className={button}
            aria-label="Always on top"
            title="Always on top"
            aria-pressed={pinned}
            onClick={() =>
              void run(async () => {
                await getCurrentWindow().setAlwaysOnTop(!pinned);
                setPinned(!pinned);
              })
            }
          >
            <Pin size={14} fill={pinned ? "currentColor" : "none"} />
          </button>
        )}
        <button
          className={button}
          aria-label={
            fullscreen ? "Collapse player" : "Expand player in main window"
          }
          title={
            fullscreen
              ? "Collapse player (Esc)"
              : "Expand player in main window"
          }
          disabled={changingWindow}
          onClick={() =>
            expanded ? onCollapse?.() : void expandInMainWindow()
          }
        >
          {fullscreen ? <ChevronDown size={20} /> : <Maximize2 size={16} />}
        </button>
        {!expanded && (
          <button
            className={button}
            aria-label="Close player window"
            title="Close player window"
            onClick={() => void run(() => getCurrentWindow().close())}
          >
            <X size={16} />
          </button>
        )}
      </header>
      <div
        className={`flex min-h-0 flex-1 ${expanded && panel ? "gap-[clamp(1.5rem,4vw,4rem)] pr-6" : ""}`}
      >
        <div
          className={
            fullscreen
              ? `flex min-h-0 flex-col justify-center overflow-y-auto px-8 pb-8 ${panel ? "w-[46%] min-w-md max-w-xl shrink-0" : "mx-auto min-w-0 w-full max-w-xl flex-1"}`
              : "flex min-h-0 min-w-0 flex-1 flex-col"
          }
        >
          <div
            className={
              fullscreen
                ? "flex min-w-0 flex-col items-center gap-6 pt-2 text-center"
                : "flex min-w-0 items-center gap-4 px-6 pt-2"
            }
          >
            <div
              className={`flex shrink-0 items-center justify-center overflow-hidden bg-linear-to-br from-muted to-primary/20 shadow-lg ${fullscreen ? "size-[min(40vh,380px)] rounded-3xl shadow-2xl" : "size-17 rounded-xl"}`}
            >
              {artwork && artwork !== failedArtwork ? (
                <img
                  src={artwork}
                  alt="Album artwork"
                  onError={() => setFailedArtwork(artwork)}
                  className="size-full object-cover"
                />
              ) : (
                <Music2
                  size={fullscreen ? 80 : 26}
                  className="text-foreground/60"
                />
              )}
            </div>
            <div className="min-w-0 max-w-full">
              <h1
                title={song?.title}
                className={`truncate font-semibold ${fullscreen ? "text-3xl" : "text-sm"}`}
              >
                {song?.title || "Nothing is playing"}
              </h1>
              <p
                title={song?.artist}
                className={`mt-1 truncate text-foreground/60 ${fullscreen ? "text-lg" : "text-xs"}`}
              >
                {song
                  ? song.artist || "Unknown artist"
                  : "Choose a song from your library"}
              </p>
              {song?.album && (
                <p
                  className={`mt-1 truncate text-foreground/40 ${fullscreen ? "text-sm" : "text-[11px]"}`}
                >
                  {song.album}
                </p>
              )}
            </div>
          </div>
          <div className={fullscreen ? "shrink-0 pt-8" : "px-6 pt-4"}>
            <SeekBar
              key={song?.id ?? 0}
              position={position}
              duration={duration}
              paused={paused}
              disabled={!song}
              onSeek={seek}
              showTimes
            />
          </div>
          <div
            className={
              fullscreen
                ? "flex shrink-0 items-center justify-center gap-10 pt-6"
                : "flex flex-1 items-center justify-center gap-6 pb-2"
            }
          >
            <button
              className={`${button} ${modes.shuffled ? "bg-primary/10" : ""}`}
              aria-label={modes.shuffled ? "Disable shuffle" : "Enable shuffle"}
              title={modes.shuffled ? "Disable shuffle" : "Enable shuffle"}
              aria-pressed={modes.shuffled}
              disabled={!song || busy}
              onClick={() => void control("toggle_shuffle")}
            >
              <Shuffle
                size={18}
                className={modes.shuffled ? "text-primary" : undefined}
              />
            </button>
            <button
              className={button}
              aria-label="Previous"
              disabled={!song || busy}
              onClick={() => void control("previous_song")}
            >
              <SkipBack size={19} fill="currentColor" />
            </button>
            <button
              className={`flex items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg ${reducedMotion ? "" : "transition-transform hover:scale-105 motion-reduce:transform-none motion-reduce:transition-none"} focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary disabled:opacity-40 ${fullscreen ? "size-16" : "size-11"}`}
              aria-label={paused || !song ? "Play" : "Pause"}
              disabled={!song || busy}
              onClick={() =>
                void control(paused ? "resume_playback" : "pause_playback")
              }
            >
              {paused || !song ? (
                <Play size={20} fill="currentColor" />
              ) : (
                <Pause size={20} fill="currentColor" />
              )}
            </button>
            <button
              className={button}
              aria-label="Next"
              disabled={!song || busy}
              onClick={() => void control("skip_song")}
            >
              <SkipForward size={19} fill="currentColor" />
            </button>
            <button
              className={`${button} ${modes.repeat_mode !== "off" ? "bg-primary/10" : ""}`}
              aria-label={repeatLabel}
              title={repeatLabel}
              aria-pressed={modes.repeat_mode !== "off"}
              disabled={!song || busy}
              onClick={() =>
                void control("set_repeat_mode", { repeatMode: nextRepeatMode })
              }
            >
              {modes.repeat_mode === "track" ? (
                <Repeat1 size={18} className="text-primary" />
              ) : (
                <Repeat
                  size={18}
                  className={
                    modes.repeat_mode === "queue" ? "text-primary" : undefined
                  }
                />
              )}
            </button>
          </div>
        </div>
        {expanded && panel && (
          <div
            id="expanded-playback-panel"
            className="flex min-h-0 min-w-0 flex-1"
          >
            <Suspense
              fallback={
                <p
                  role="status"
                  className="w-80 p-6 text-sm text-muted-foreground"
                >
                  Loading panel…
                </p>
              }
            >
              <PlaybackPanel
                embedded
                view={panel}
                onViewChange={setPanel}
                onClose={closePanel}
                fileId={song?.id ?? 0}
                title={song?.title}
                artist={song?.artist}
                position={position * 1000}
                onSeek={(milliseconds) => seek(milliseconds / 1000)}
                onPlay={(index) =>
                  void run(() => invoke("skip_to_index", { index }))
                }
              />
            </Suspense>
          </div>
        )}
      </div>
    </main>
  );
}
