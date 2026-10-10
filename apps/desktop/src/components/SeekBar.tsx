import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent,
} from "react";
import { cn } from "../utils";

type SeekBarProps = {
  position: number;
  duration: number;
  paused: boolean;
  disabled?: boolean;
  onSeek: (seconds: number) => void;
  className?: string;
  compact?: boolean;
  showTimes?: boolean;
  onActivateHover?: () => void;
};

export function formatSeekTime(seconds: number) {
  const total = Number.isFinite(seconds) ? Math.max(0, Math.floor(seconds)) : 0;
  return `${Math.floor(total / 60)}:${(total % 60).toString().padStart(2, "0")}`;
}

export default function SeekBar({
  position,
  duration,
  paused,
  disabled = false,
  onSeek,
  className,
  compact = false,
  showTimes = false,
  onActivateHover,
}: SeekBarProps) {
  const [draft, setDraft] = useState<number | null>(null);
  const pointerActive = useRef<number | null>(null);
  const dragValue = useRef(0);
  const [pendingSeek, setPendingSeek] = useState<number | null>(null);
  const maximum = Number.isFinite(duration) ? Math.max(0, duration) : 0;
  const inactive = disabled || maximum === 0;
  const confirmed = Number.isFinite(position) ? Math.max(0, position) : 0;
  const [displayPosition, setDisplayPosition] = useState(confirmed);
  const displayed = useRef(confirmed);
  const anchor = useRef({ position: confirmed, time: performance.now() });

  useEffect(() => {
    if (pendingSeek === null) return;
    const timeout = setTimeout(() => setPendingSeek(null), 1000);
    return () => clearTimeout(timeout);
  }, [pendingSeek]);

  useEffect(() => {
    if (pendingSeek !== null && Math.abs(confirmed - pendingSeek) > 0.75)
      return;
    if (pendingSeek !== null) setPendingSeek(null);
    anchor.current = { position: confirmed, time: performance.now() };
    if (paused || inactive || Math.abs(confirmed - displayed.current) > 0.75) {
      displayed.current = confirmed;
      setDisplayPosition(confirmed);
    }
  }, [confirmed, paused, inactive, pendingSeek]);

  useEffect(() => {
    if (paused || inactive || draft !== null) return;
    let frame = 0;
    let lastTime = performance.now();
    function tick(now: number) {
      const elapsed = Math.min(0.1, (now - lastTime) / 1000);
      lastTime = now;
      const age = Math.max(0, (now - anchor.current.time) / 1000);
      const target = anchor.current.position + Math.min(age, 0.5);
      const advanced = displayed.current + (age < 0.5 ? elapsed : 0);
      displayed.current = Math.min(
        maximum,
        Math.max(
          0,
          advanced + (target - advanced) * (1 - Math.exp(-elapsed * 12)),
        ),
      );
      setDisplayPosition(displayed.current);
      frame = requestAnimationFrame(tick);
    }
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [paused, inactive, maximum, draft]);

  const value = Math.min(maximum, Math.max(0, draft ?? displayPosition));
  const progress = maximum ? (value / maximum) * 100 : 0;

  function commit(value: number) {
    pointerActive.current = null;
    setDraft(null);
    setPendingSeek(value);
    displayed.current = value;
    setDisplayPosition(value);
    anchor.current = { position: value, time: performance.now() };
    if (!inactive) onSeek(value);
  }

  function previewPointer(event: PointerEvent<HTMLInputElement>) {
    const input = event.currentTarget;
    const bounds = input.getBoundingClientRect();
    const travel = Math.max(1, bounds.width - 12);
    const fraction = Math.min(
      1,
      Math.max(0, (event.clientX - bounds.left - 6) / travel),
    );
    const next = fraction * maximum;
    dragValue.current = next;
    input.value = String(next);
    input.style.setProperty("--seek-progress", `${fraction * 100}%`);
    setDraft(next);
    return next;
  }

  function cancelDrag() {
    pointerActive.current = null;
    setDraft(null);
  }

  return (
    <div
      className={cn(
        "seek-control w-full min-w-0 select-none",
        compact && "seek-control--compact",
        className,
      )}
      onMouseEnter={onActivateHover}
      onFocus={onActivateHover}
    >
      <input
        type="range"
        className="player-seek"
        style={{ "--seek-progress": `${progress}%` } as CSSProperties}
        min={0}
        max={maximum}
        step="any"
        value={value}
        disabled={inactive}
        aria-label="Seek position"
        aria-valuetext={`${formatSeekTime(value)} of ${formatSeekTime(maximum)}`}
        onKeyDown={(event) => {
          const targets: Record<string, number> = {
            ArrowLeft: value - 5,
            ArrowDown: value - 5,
            ArrowRight: value + 5,
            ArrowUp: value + 5,
            PageDown: value - 10,
            PageUp: value + 10,
            Home: 0,
            End: maximum,
          };
          if (event.key in targets) {
            event.preventDefault();
            commit(Math.min(maximum, Math.max(0, targets[event.key])));
          }
        }}
        onPointerDown={(event) => {
          if (inactive || !event.isPrimary || event.button !== 0) return;
          event.preventDefault();
          event.currentTarget.focus({ preventScroll: true });
          pointerActive.current = event.pointerId;
          event.currentTarget.setPointerCapture(event.pointerId);
          previewPointer(event);
        }}
        onPointerMove={(event) => {
          if (pointerActive.current === event.pointerId) previewPointer(event);
        }}
        onChange={(event) => {
          if (pointerActive.current === null)
            commit(Number(event.currentTarget.value));
        }}
        onPointerUp={(event) => {
          if (pointerActive.current !== event.pointerId) return;
          commit(previewPointer(event));
          event.currentTarget.releasePointerCapture(event.pointerId);
        }}
        onPointerCancel={cancelDrag}
        onLostPointerCapture={() => {
          if (pointerActive.current !== null) cancelDrag();
        }}
        onBlur={() => {
          if (pointerActive.current !== null) commit(dragValue.current);
        }}
      />
      {showTimes && (
        <div className="flex justify-between text-[10px] font-medium tabular-nums text-foreground/50">
          <span>{formatSeekTime(value)}</span>
          <span>{formatSeekTime(maximum)}</span>
        </div>
      )}
    </div>
  );
}
