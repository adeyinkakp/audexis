import type { DatabaseMediaFile } from "../../hooks/useFileWatcher";
import { formatDuration } from "../../utils/duration";

export type SongRow = DatabaseMediaFile & {
  title: string;
  artist: string;
  album: string;
  genre: string;
};
export const songColumns = [
  { id: "title", label: "Title", width: 240 },
  { id: "artist", label: "Artist", width: 180 },
  { id: "album", label: "Album", width: 200 },
  { id: "duration_ms", label: "Time", width: 90 },
  { id: "genre", label: "Genre", width: 140 },
  { id: "format", label: "Format", width: 100 },
  { id: "file_name", label: "File name", width: 220 },
  { id: "path", label: "Path", width: 320 },
  { id: "size", label: "Size", width: 110 },
  { id: "modified_at", label: "Date modified", width: 180 },
] as const;
export type SongColumnId = (typeof songColumns)[number]["id"];
export const defaultColumnOrder: SongColumnId[] = [
  "duration_ms",
  "format",
  "path",
  "file_name",
  "title",
  "album",
  "genre",
];
export const MIN_COLUMN_WIDTH = 72;
export const MAX_COLUMN_WIDTH = 1200;

export function displaySongValue(id: SongColumnId, value: unknown): string {
  if (id === "duration_ms") return formatDuration(value as number | null);
  if (id === "size")
    return typeof value === "number"
      ? `${(value / 1024 / 1024).toFixed(1)} MB`
      : "—";
  if (id === "modified_at")
    return typeof value === "number" && value > 0
      ? new Date(value * 1000).toLocaleString()
      : "—";
  return String(value ?? "—");
}
