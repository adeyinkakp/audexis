import { AnimatePresence, motion } from "motion/react";

// Apple music inspiration
export default function LyricsGap({
  position,
  start,
  end,
  active,
  reducedMotion,
}: {
  position: number;
  start: number;
  end?: number;
  active: boolean;
  reducedMotion: boolean;
}) {
  const visible =
    active && position >= start && (end === undefined || position < end);

  const elapsed = Math.max(0, position - start);
  const progress =
    end !== undefined && end > start
      ? Math.min(1, elapsed / (end - start))
      : (elapsed % 2400) / 2400;
  return (
    <AnimatePresence initial={false}>
      {visible && (
        <motion.span
          key="gap"
          initial={{ opacity: 0, y: reducedMotion ? 0 : 14 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: reducedMotion ? 0 : -10 }}
          transition={
            reducedMotion
              ? { duration: 0 }
              : { duration: 0.35, ease: [0.22, 1, 0.36, 1] }
          }
          className="inline-flex h-[1.3em] items-center gap-[0.22em] align-middle"
        >
          <span className="sr-only">Instrumental break</span>
          {[0, 1, 2].map((dot) => {
            const fill = Math.max(0, Math.min(1, progress * 3 - dot));
            return (
              <motion.span
                key={dot}
                aria-hidden="true"
                className="block size-[0.3em] rounded-full bg-current"
                animate={{
                  opacity: active ? 0.3 + fill * 0.7 : 0.5,
                  scale:
                    active && !reducedMotion
                      ? 1 + Math.sin(fill * Math.PI) * 0.3
                      : 1,
                }}
                transition={
                  reducedMotion
                    ? { duration: 0 }
                    : { duration: 0.15, ease: "linear" }
                }
              />
            );
          })}
        </motion.span>
      )}
    </AnimatePresence>
  );
}
