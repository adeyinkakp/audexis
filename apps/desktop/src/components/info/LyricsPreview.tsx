import LyricsGap from "./LyricsGap";
import { motion, useReducedMotion } from "motion/react";
import { useEffect, useRef } from "react";
import { useStore } from "../../hooks/useStore";
import { cn } from "../../utils";
import { activeLyricIndex, type LyricRow } from "./synchronizedLyrics";

export default function LyricsPreview({
  rows,
  position,
  onSeek,
  active = true,
  disabled = false,
  compact = false,
  immersive = false,
  className,
  plainText = "",
}: {
  rows: LyricRow[];
  position: number;
  onSeek: (milliseconds: number) => void;
  active?: boolean;
  disabled?: boolean;
  compact?: boolean;
  immersive?: boolean;
  className?: string;
  plainText?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const { preferences } = useStore();
  const systemReduced = useReducedMotion();
  const reduced = preferences.reduceMotion || systemReduced;
  const current = activeLyricIndex(rows, position);
  useEffect(() => {
    if (!active) return;
    const container = ref.current;
    const line = container?.querySelector<HTMLElement>('[aria-current="true"]');
    if (container && line)
      container.scrollTo({
        top:
          line.offsetTop - container.clientHeight / 2 + line.clientHeight / 2,
        behavior: reduced ? "instant" : "smooth",
      });
  }, [current, active, reduced, rows]);
  return (
    <div
      ref={ref}
      aria-label="Lyrics preview"
      style={{
        fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
      }}
      className={cn(
        "relative min-h-0 overflow-y-auto overscroll-contain px-6 py-8 scrollbar-none [&::-webkit-scrollbar]:hidden [-webkit-mask-image:linear-gradient(transparent,black_7%,black_90%,transparent)]",
        className,
      )}
    >
      {rows.length ? (
        rows.map((row, index) => {
          const gap = !row.text.trim();
          const visible = !gap || (active && index === current);
          return (
            <motion.button
              key={`${index}-${row.time}`}
              type="button"
              initial={false}
              aria-hidden={!visible || undefined}
              tabIndex={visible ? 0 : -1}
              aria-current={index === current ? "true" : undefined}
              disabled={disabled}
              onClick={() => onSeek(row.time)}
              animate={{
                height: visible ? "auto" : 0,
                marginBottom: visible ? (immersive ? 32 : 20) : 0,
                opacity: !visible ? 0 : index === current ? 1 : 0.32,
                scale: index === current ? 1 : 0.98,
                filter:
                  index !== current && !reduced ? "blur(0.5px)" : "blur(0px)",
              }}
              whileHover={{ opacity: 1, filter: "blur(0px)" }}
              transition={
                reduced
                  ? { duration: 0 }
                  : { type: "spring", stiffness: 220, damping: 30 }
              }
              className={cn(
                "block w-full overflow-hidden origin-left wrap-break-word text-left font-bold leading-[1.3] tracking-tight outline-none focus-visible:rounded focus-visible:ring-2 focus-visible:ring-current",
                immersive ? "text-[clamp(2rem,3.5vw,3.5rem)]" : compact ? "text-2xl" : "text-[clamp(1.5rem,3vw,2.25rem)]",
              )}
            >
              {row.text.trim() ? (
                row.text
              ) : (
                <LyricsGap
                  position={position}
                  start={row.time}
                  end={
                    rows.slice(index + 1).find((line) => line.text.trim())?.time
                  }
                  active={active && index === current}
                  reducedMotion={!!reduced}
                />
              )}
            </motion.button>
          );
        })
      ) : plainText ? (
        <p className={cn("whitespace-pre-wrap wrap-break-word font-semibold leading-relaxed opacity-80", immersive ? "text-[clamp(2rem,3.5vw,3.5rem)]" : "text-xl")}>
          {plainText}
        </p>
      ) : (
        <p className="text-sm opacity-60">No synchronized lyrics yet.</p>
      )}
      {rows.length > 0 && <div className="h-24" aria-hidden="true" />}
    </div>
  );
}
