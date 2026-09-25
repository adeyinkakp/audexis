import { useEffect, useState } from "react";
import { cn } from "../utils";

type SeekBarProps = {
  position: number;
  duration: number;
  disabled?: boolean;
  onSeek: (seconds: number) => void;
  className?: string;
  compact?: boolean;
  onActivateHover?: () => void;
};

function formatTime(seconds: number) {
  const totalSeconds = Math.max(0, Math.floor(seconds));
  const minutes = Math.floor(totalSeconds / 60);
  const remainingSeconds = totalSeconds % 60;
  return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
}

export default function SeekBar({
  position,
  duration,
  disabled = false,
  onSeek,
  className,
  compact = false,
  onActivateHover,
}: SeekBarProps) {
  const [draftPosition, setDraftPosition] = useState(position);
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    if (!dragging) setDraftPosition(position);
  }, [dragging, position]);

  const value = Math.min(
    Math.max(0, dragging ? draftPosition : position),
    duration,
  );
  const progressPercent = duration > 0 ? (value / duration) * 100 : 0;

  return (
    <div
      className={cn(
        "group/seekbar flex w-full items-center select-none",
        compact ? "gap-0" : "gap-2",
        className,
      )}
      onMouseEnter={onActivateHover}
      onFocus={onActivateHover}
    >
      <div
        className={cn(
          "relative flex min-w-24 flex-1 items-center",
          compact ? "h-2.5 sm:h-3" : "h-7",
        )}
      >
        <div
          aria-hidden="true"
          className={cn(
            "pointer-events-none relative w-full overflow-hidden rounded-full bg-foreground/20 transition-[height] duration-200 ease-out",
            compact
              ? "h-0.75 group-hover/seekbar:h-1.75 group-focus-within/seekbar:h-1.75 sm:h-1 sm:group-hover/seekbar:h-2 sm:group-focus-within/seekbar:h-2"
              : "h-1 group-hover/seekbar:h-2 group-focus-within/seekbar:h-2",
          )}
        >
          <div
            className={cn(
              "h-full rounded-full bg-primary transition-[width]",
              dragging ? "duration-0" : "duration-150",
            )}
            style={{ width: `${progressPercent}%` }}
          />
        </div>

        <div
          aria-hidden="true"
          className={cn(
            "pointer-events-none absolute top-1/2 -ml-1 h-2.5 w-2.5 -translate-y-1/2 rounded-full bg-white shadow-[0_1px_4px_rgba(0,0,0,0.35)] transition-all duration-150 ease-out sm:-ml-1.5 sm:h-3 sm:w-3",
            dragging
              ? "scale-125 opacity-100"
              : "scale-75 opacity-0 group-hover/seekbar:scale-100 group-hover/seekbar:opacity-100 group-focus-within/seekbar:scale-100 group-focus-within/seekbar:opacity-100",
          )}
          style={{ left: `${progressPercent}%` }}
        />

        <input
          type="range"
          min={0}
          max={Math.max(duration, 0)}
          step={0.1}
          value={value}
          disabled={disabled}
          aria-label="Seek position"
          onPointerDown={() => setDragging(true)}
          onPointerUp={() => {
            setDragging(false);
            onSeek(draftPosition);
          }}
          onChange={(event) => {
            const nextPosition = Number(event.target.value);
            setDraftPosition(nextPosition);
          }}
          className="absolute inset-0 z-10 h-full w-full cursor-pointer appearance-none bg-transparent opacity-0 disabled:cursor-default"
        />
      </div>
    </div>
  );
}

export function formatSeekTime(seconds: number) {
  return formatTime(seconds);
}
