import LyricsGap from "./LyricsGap";
import LyricsPreview from "./LyricsPreview";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
// i love framer motion
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import {
  Clock3,
  Code2,
  Minus,
  Music2,
  Pause,
  Play,
  Plus,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import TextareaAutosize from "react-textarea-autosize";
import { useLyricsPreview } from "../../hooks/useLyricsPreview";
import { useStore } from "../../hooks/useStore";
import { cn } from "../../utils";
import {
  activeLyricIndex,
  formatTimestamp,
  parseLyrics,
  parseTimestamp,
  serializeLyrics,
  type LyricRow,
} from "./synchronizedLyrics";

type EditorRow = LyricRow & { id: string };
const smallButton =
  "inline-flex size-7 shrink-0 items-center justify-center rounded-full text-white/60 transition-colors hover:bg-white/15 hover:text-white focus-visible:outline-2 focus-visible:outline-white disabled:opacity-25";

export default function SynchronizedLyricsEditor({
  value,
  onChange,
  fileId,
  title,
  artist,
  disabled,
  active,
}: {
  value: string;
  onChange: (value: string) => void;
  fileId: number;
  title: string;
  artist: string;
  disabled: boolean;
  active: boolean;
}) {
  const [mode, setMode] = useState<"editor" | "preview" | "raw">("editor");
  const [editingId, setEditingId] = useState<string | null>(null);
  const { preferences } = useStore();
  const systemReducedMotion = useReducedMotion();
  const reduceMotion = preferences.reduceMotion || systemReducedMotion;
  const transition = reduceMotion
    ? { duration: 0 }
    : { type: "spring" as const, stiffness: 220, damping: 30 };
  const preview = useLyricsPreview(fileId, active && !disabled);
  const {
    position,
    duration,
    playing,
    error: audioError,
    play,
    pause,
    seek,
    stamp,
  } = preview;
  const artwork = useQuery({
    queryKey: ["libraryArtwork", fileId],
    queryFn: () => invoke<string | null>("get_artwork", { fileId }),
  });
  const parsed = useMemo(() => parseLyrics(value), [value]);

  const draft = useRef<{ value: string; rows: EditorRow[] }>({
    value: "",
    rows: [],
  });
  if (draft.current.value !== value) {
    const unused = [...draft.current.rows];
    draft.current = {
      value,
      rows: parsed.rows.map((row) => {
        const match = unused.findIndex(
          (old) => old.time === row.time && old.text === row.text,
        );
        return {
          ...row,
          id: match < 0 ? crypto.randomUUID() : unused.splice(match, 1)[0].id,
        };
      }),
    };
  }
  const rows = draft.current.rows;
  const current = activeLyricIndex(rows, position);

  const commit = (next: EditorRow[]) => {
    const sorted = [...next].sort((a, b) => a.time - b.time);
    const text = serializeLyrics(sorted);
    draft.current = { value: text, rows: sorted };
    onChange(text);
  };
  const update = (id: string, patch: Partial<LyricRow>) => {
    commit(rows.map((row) => (row.id === id ? { ...row, ...patch } : row)));
  };
  const nudge = (row: EditorRow, offset: number) => {
    update(row.id, {
      time: Math.max(0, Math.min(duration || 0xffffffff, row.time + offset)),
    });
  };
  const add = () => {
    const id = crypto.randomUUID();
    commit([...rows, { id, time: stamp(), text: "" }]);
    setEditingId(id);
  };

  return (
    <section
      style={{
        fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
      }}
      aria-label="Synchronized lyrics editor"
      className="relative isolate flex min-h-[440px] flex-col overflow-hidden rounded-2xl bg-[#211b27] text-white shadow-xl shadow-black/10"
    >
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 -z-10 overflow-hidden"
      >
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_15%_0%,#8c525b_0%,transparent_65%),radial-gradient(ellipse_at_100%_90%,#59456d_0%,transparent_70%)]" />
        {artwork.data && (
          <motion.img
            src={artwork.data}
            alt=""
            className="absolute -inset-[20%] size-[140%] max-w-none object-cover opacity-55 blur-[70px]"
            animate={
              reduceMotion || !active
                ? { scale: 1.1 }
                : { scale: [1.1, 1.25, 1.1], rotate: [0, 8, 0] }
            }
            transition={{ duration: 30, repeat: Infinity, ease: "easeInOut" }}
          />
        )}
        <div className="absolute inset-0 bg-black/35" />
      </div>

      <header className="flex flex-wrap items-center justify-between gap-3 px-6 pb-2 pt-5">
        <div
          aria-label="Lyrics view"
          className="flex gap-1 rounded-full bg-black/15 p-1"
        >
          {(
            [
              ["editor", "Edit"],
              ["preview", "Preview"],
              ["raw", "Raw LRC"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              aria-pressed={mode === id}
              onClick={() => setMode(id)}
              className={cn(
                "relative rounded-full px-3 py-1.5 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-white",
                mode === id ? "text-white" : "text-white/50 hover:text-white",
              )}
            >
              {mode === id && (
                <motion.span
                  layoutId={`lyrics-mode-${fileId}`}
                  className="absolute inset-0 rounded-full bg-white/15 shadow-sm"
                  transition={transition}
                />
              )}
              <span className="relative flex items-center gap-1.5">
                {id === "raw" && <Code2 size={12} />}
                {label}
              </span>
            </button>
          ))}
        </div>
      </header>

      {parsed.error && (
        <p
          role="alert"
          className="mx-6 my-3 rounded-xl bg-black/20 p-3 text-xs text-rose-200"
        >
          {parsed.error}
        </p>
      )}
      <AnimatePresence mode="wait" initial={false}>
        {mode === "raw" ? (
          <motion.div
            key="raw"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={transition}
            className="flex flex-1 px-6 py-4"
          >
            <textarea
              aria-label="Synchronized lyrics (LRC)"
              spellCheck={false}
              value={value}
              disabled={disabled}
              onChange={(event) => onChange(event.target.value)}
              placeholder="[00:12.500]First line\n[00:17.250]Next line"
              className="min-h-64 w-full resize-y rounded-xl border border-white/10 bg-black/15 p-4 font-mono text-sm leading-7 text-white/85 outline-none placeholder:text-white/30 focus:border-white/40 disabled:opacity-50"
            />
          </motion.div>
        ) : mode === "preview" && !parsed.error ? (
          <LyricsPreview
            key="preview"
            rows={parsed.rows}
            position={position}
            active={active}
            disabled={disabled || !preview.ready}
            onSeek={(time) => {
              void seek(time).then(play);
            }}
            className="h-[34vh] min-h-64 shrink-0 md:px-10"
          />
        ) : (
          !parsed.error && (
            <motion.div
              key="lyrics"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={transition}
              className="relative h-[34vh] min-h-64 shrink-0 overflow-y-auto overscroll-contain px-6 py-8 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden [-webkit-mask-image:linear-gradient(transparent,black_7%,black_90%,transparent)] md:px-10"
            >
              {!rows.length && (
                <div className="flex flex-col gap-3 pb-4">
                  <p className="max-w-xs text-sm leading-relaxed text-white/50">
                    Play your track and add a line as it begins. Already have
                    timed lyrics? Paste them in Raw LRC tab
                  </p>
                </div>
              )}
              <AnimatePresence initial={false}>
                {rows.map((row, index) => {
                  const selected = editingId === row.id && mode === "editor";
                  const highlighted = index === current;
                  return (
                    <motion.div
                      key={row.id}
                      layout="position"
                      data-lyric-row={row.id}
                      aria-current={highlighted ? "true" : undefined}
                      initial={{ opacity: 0, y: reduceMotion ? 0 : 12 }}
                      exit={{ opacity: 0, height: 0 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={transition}
                      className="group relative pb-5"
                    >
                      <AnimatePresence initial={false}>
                        {mode === "editor" && (
                          <motion.div
                            initial={{ opacity: 0, height: 0 }}
                            animate={{
                              opacity: selected ? 1 : 0.65,
                              height: "auto",
                            }}
                            exit={{ opacity: 0, height: 0 }}
                            transition={transition}
                            className="mb-1.5 flex items-center gap-1"
                          >
                            <TimestampInput
                              time={row.time}
                              label={`Timestamp for line ${index + 1}`}
                              disabled={disabled}
                              onChange={(time) => update(row.id, { time })}
                            />
                            <span className="mx-1 h-3 w-px bg-white/15" />
                            <button
                              type="button"
                              className={smallButton}
                              aria-label={`Move line ${index + 1} earlier by 250 milliseconds`}
                              title="250 ms earlier"
                              disabled={disabled || row.time === 0}
                              onClick={() => nudge(row, -250)}
                            >
                              <Minus size={13} />
                            </button>
                            <button
                              type="button"
                              className={smallButton}
                              aria-label={`Move line ${index + 1} later by 250 milliseconds`}
                              title="250 ms later"
                              disabled={
                                disabled ||
                                (duration > 0 && row.time >= duration)
                              }
                              onClick={() => nudge(row, 250)}
                            >
                              <Plus size={13} />
                            </button>
                            <button
                              type="button"
                              className={smallButton}
                              aria-label={`Set line ${index + 1} to current playback time`}
                              title="Use current playback time"
                              disabled={disabled || !preview.ready}
                              onClick={() => update(row.id, { time: stamp() })}
                            >
                              <Clock3 size={13} />
                            </button>
                            <button
                              type="button"
                              className={`${smallButton} ml-1`}
                              aria-label={`Remove line ${index + 1}`}
                              disabled={disabled}
                              onClick={() =>
                                commit(
                                  rows.filter((item) => item.id !== row.id),
                                )
                              }
                            >
                              <Trash2 size={13} />
                            </button>
                          </motion.div>
                        )}
                      </AnimatePresence>
                      {selected ? (
                        <TextareaAutosize
                          autoFocus
                          aria-label={`Lyric line ${index + 1}`}
                          value={row.text}
                          disabled={disabled}
                          onChange={(event) =>
                            update(row.id, {
                              text: event.target.value.replace(/[\r\n]+/g, " "),
                            })
                          }
                          onKeyDown={(event) => {
                            if (
                              event.key === "Enter" &&
                              !event.nativeEvent.isComposing
                            ) {
                              event.preventDefault();
                              setEditingId(null);
                            }
                          }}
                          placeholder="Type a lyric…"
                          className="block w-full resize-none bg-transparent text-[clamp(1.5rem,3vw,2.25rem)] font-bold leading-[1.3] tracking-tight text-white outline-none placeholder:text-white/30"
                        />
                      ) : (
                        <motion.button
                          type="button"
                          aria-label={
                            !row.text.trim()
                              ? `Edit instrumental break at ${formatTimestamp(row.time)}`
                              : undefined
                          }
                          disabled={disabled}
                          onClick={() => {
                            setEditingId(row.id);
                          }}
                          animate={{
                            opacity: highlighted
                              ? 1
                              : mode === "editor"
                                ? 0.52
                                : 0.32,
                            scale: highlighted ? 1 : 0.98,
                            filter: "blur(0px)",
                          }}
                          whileHover={{ opacity: 1, filter: "blur(0px)" }}
                          transition={transition}
                          className="block min-h-[1.3em] w-full origin-left break-words text-left text-[clamp(1.5rem,3vw,2.25rem)] font-bold leading-[1.3] tracking-tight outline-none focus-visible:rounded focus-visible:ring-2 focus-visible:ring-white/50"
                        >
                          {row.text.trim() ? (
                            row.text
                          ) : (
                            <LyricsGap
                              position={position}
                              start={row.time}
                              end={
                                rows
                                  .slice(index + 1)
                                  .find((line) => line.text.trim())?.time
                              }
                              active={active && highlighted}
                              reducedMotion={!!reduceMotion}
                            />
                          )}
                        </motion.button>
                      )}
                    </motion.div>
                  );
                })}
              </AnimatePresence>
              {mode === "editor" && (
                <motion.button
                  layout
                  type="button"
                  disabled={disabled}
                  onClick={add}
                  whileTap={reduceMotion ? {} : { scale: 0.96 }}
                  className="mt-1 inline-flex items-center gap-2 rounded-full bg-white/10 px-4 py-2.5 text-sm font-medium text-white/80 transition-colors hover:bg-white/20 disabled:opacity-40"
                >
                  <Plus size={16} /> Add line at current time
                </motion.button>
              )}
            </motion.div>
          )
        )}
      </AnimatePresence>

      <footer className="mx-6 border-t border-white/10 pb-5 pt-4">
        <div className="flex items-center gap-4">
          <div className="flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-white/10 shadow-lg">
            {artwork.data ? (
              <img
                src={artwork.data}
                alt="Album artwork"
                className="size-full object-cover"
              />
            ) : (
              <Music2 size={22} className="text-white/50" />
            )}
          </div>
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold">{title}</p>
            <p className="mt-0.5 truncate text-xs text-white/50">
              {artist || "Unknown artist"}
            </p>
          </div>
          <motion.button
            type="button"
            disabled={!preview.ready || disabled}
            onClick={() => (playing ? void pause() : void play())}
            aria-label={
              playing ? "Pause lyrics preview" : "Play lyrics preview"
            }
            whileTap={reduceMotion ? {} : { scale: 0.88 }}
            className="flex size-11 items-center justify-center rounded-full text-white transition-colors hover:bg-white/10 disabled:opacity-30"
          >
            <AnimatePresence mode="wait" initial={false}>
              <motion.span
                key={playing ? "pause" : "play"}
                initial={{ opacity: 0, scale: reduceMotion ? 1 : 0.7 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: reduceMotion ? 1 : 0.7 }}
                transition={{ duration: reduceMotion ? 0 : 0.12 }}
              >
                {playing ? (
                  <Pause size={25} fill="currentColor" />
                ) : (
                  <Play size={25} fill="currentColor" />
                )}
              </motion.span>
            </AnimatePresence>
          </motion.button>
        </div>
        <input
          type="range"
          aria-label="Lyrics preview position"
          min={0}
          max={duration || 1}
          step={10}
          value={position}
          disabled={!preview.ready || !duration || disabled}
          onChange={(event) => void seek(Number(event.target.value))}
          style={{
            background: `linear-gradient(to right, rgba(255,255,255,.75) ${duration ? (position / duration) * 100 : 0}%, rgba(255,255,255,.18) ${duration ? (position / duration) * 100 : 0}%)`,
          }}
          className="mt-4 block h-1 w-full cursor-pointer appearance-none rounded-full disabled:opacity-30 [&::-webkit-slider-thumb]:size-2 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-moz-range-thumb]:size-2 [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-white"
        />
        <div className="mt-2 flex justify-between font-mono text-[10px] tabular-nums text-white/40">
          <span>{formatTimestamp(position)}</span>
          <span>−{formatTimestamp(Math.max(0, duration - position))}</span>
        </div>
        {!preview.ready && !audioError && (
          <p role="status" className="mt-2 text-xs text-white/50">
            Loading preview…
          </p>
        )}
        {audioError && (
          <p role="status" className="mt-2 text-xs text-rose-200">
            {audioError}{" "}
            <button
              type="button"
              onClick={preview.retry}
              className="ml-2 underline"
            >
              Retry preview
            </button>
          </p>
        )}
      </footer>
    </section>
  );
}

function TimestampInput({
  time,
  label,
  disabled,
  onChange,
}: {
  time: number;
  label: string;
  disabled: boolean;
  onChange: (time: number) => void;
}) {
  const [text, setText] = useState(formatTimestamp(time));
  useEffect(() => setText(formatTimestamp(time)), [time]);
  const invalid = parseTimestamp(text) === null;
  return (
    <input
      aria-label={label}
      aria-invalid={invalid}
      title={invalid ? "Use mm:ss.xxx" : "Edit timestamp (mm:ss.xxx)"}
      value={text}
      disabled={disabled}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => {
        const next = parseTimestamp(text);
        if (next !== null && next !== time) onChange(next);
        else setText(formatTimestamp(time));
      }}
      onKeyDown={(event) => {
        if (event.key === "Enter") event.currentTarget.blur();
      }}
      className={cn(
        "w-22 rounded bg-transparent py-1 font-mono text-[11px] tabular-nums text-white/70 outline-none focus:bg-white/10 focus:text-white",
        invalid && "text-rose-300",
      )}
    />
  );
}
