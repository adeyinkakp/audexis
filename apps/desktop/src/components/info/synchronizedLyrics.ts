export type LyricRow = { time: number; text: string };

export function parseTimestamp(value: string): number | null {
  const match = /^(\d+):(\d{2})(?:\.(\d{1,3}))?$/.exec(value);
  if (!match || Number(match[2]) >= 60) return null;
  const time =
    Number(match[1]) * 60_000 +
    Number(match[2]) * 1000 +
    Number((match[3] ?? "").padEnd(3, "0"));
  return Number.isSafeInteger(time) && time <= 0xffffffff ? time : null;
}

export function formatTimestamp(time: number): string {
  const ms = Math.max(0, Math.min(0xffffffff, Math.round(time)));
  return `${String(Math.floor(ms / 60_000)).padStart(2, "0")}:${String(Math.floor(ms / 1000) % 60).padStart(2, "0")}.${String(ms % 1000).padStart(3, "0")}`;
}

export function parseLyrics(value: string): {
  rows: LyricRow[];
  error: string | null;
} {
  const rows: LyricRow[] = [];
  for (const [index, line] of value.split(/\r?\n/).entries()) {
    if (!line.trim()) continue;
    const match = /^\[([^\]]+)\](.*)$/.exec(line);
    const time = match ? parseTimestamp(match[1]) : null;
    if (time === null || !match || line.includes("\0")) {
      return {
        rows: [],
        error: `Line ${index + 1} needs a timestamp like [00:12.500]. Fix it in Raw LRC to use the editor.`,
      };
    }
    if (rows.length && time < rows[rows.length - 1].time) {
      return {
        rows: [],
        error: `Line ${index + 1} is out of time order. Fix it in Raw LRC to use the editor.`,
      };
    }
    rows.push({ time, text: match[2] });
  }
  return { rows, error: null };
}

export function serializeLyrics(rows: LyricRow[]): string {
  return [...rows]
    .sort((a, b) => a.time - b.time)
    .map((row) => `[${formatTimestamp(row.time)}]${row.text}`)
    .join("\n");
}

export function activeLyricIndex(rows: LyricRow[], time: number): number {
  let active = -1;
  for (let index = 0; index < rows.length; index++) {
    if (rows[index].time > time) break;
    active = index;
  }
  return active;
}
