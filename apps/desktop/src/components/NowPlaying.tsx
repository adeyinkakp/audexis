import { SongContextMenu } from "./SongContextMenu";
import { HeartButton } from "./HeartButton";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import {
  ListMusic,
  Pause,
  Play,
  Repeat,
  Shuffle,
  SkipBack,
  SkipForward,
  Square,
} from "lucide-react";
import SeekBar, { formatSeekTime } from "./SeekBar";
import { cn } from "../utils";
import QueuePanel from "./QueuePanel";
import MarqueeText from "./MarqueeText";
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
  const [showQueue, setShowQueue] = useState(false);
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
                <HeartButton fileId={currentTrackId} />
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
            compact
            className="w-full"
            position={position}
            duration={duration}
            disabled={!song}
            onActivateHover={() => setShowSeekOverlay(true)}
            onSeek={(seconds) => {
              setPosition(seconds);
              void invoke("seek_playback", { seconds: Math.trunc(seconds) });
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
            title="Stop"
            disabled={!song}
            onClick={() => void control("stop_playback")}
          >
            <Square size={14} />
          </ControlButton>
          <ControlButton
            title="Queue"
            active={showQueue}
            onClick={() => setShowQueue((visible) => !visible)}
          >
            <ListMusic size={15} />
          </ControlButton>
        </div>
      </div>
      {showQueue && (
        <QueuePanel
          onPlay={(index) =>
            void invoke("skip_to_index", { index }).then(() => setPaused(false))
          }
        />
      )}
    </div>
  );
}
