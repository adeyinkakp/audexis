export function formatDuration(durationMs: number | null | undefined): string {
  if (durationMs == null || !Number.isFinite(durationMs) || durationMs <= 0)
    return "—";
  const totalSeconds = Math.floor(durationMs / 1000);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  const minutes = Math.floor(totalSeconds / 60);
  return minutes >= 60
    ? `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, "0")}:${seconds}`
    : `${minutes}:${seconds}`;
}

export function formatDurationInWords(durationMs: number): string {
  if (!Number.isFinite(durationMs) || durationMs < 0) return "—";
  const totalSeconds = Math.floor(durationMs / 1000);
  const units: [number, string][] = [
    [Math.floor(totalSeconds / 86400), "day"],
    [Math.floor(totalSeconds / 3600) % 24, "hour"],
    [Math.floor(totalSeconds / 60) % 60, "minute"],
  ];
  const firstNonzero = units.findIndex(([value]) => value > 0);
  return units
    .slice(firstNonzero === -1 ? units.length - 1 : firstNonzero)
    .map(([value, unit]) => `${value} ${unit}${value === 1 ? "" : "s"}`)
    .join(", ");
}
