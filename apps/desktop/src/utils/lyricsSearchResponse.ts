export type LyricsSearchResult = {
  id: number;
  trackName: string;
  artistName: string;
  albumName: string;
  duration: number | null;
  instrumental: boolean;
  plainLyrics: string | null;
  syncedLyrics: string | null;
};

export function parseLyricsSearchResponse(body: unknown): LyricsSearchResult[] {
  if (!Array.isArray(body))
    throw new Error(
      "LRCLIB returned an unexpected search response. Please try again.",
    );
  const results: LyricsSearchResult[] = [];
  for (const entry of body) {
    if (!entry || typeof entry !== "object") continue;
    const trackName =
      typeof entry.trackName === "string" ? entry.trackName : entry.name;
    if (
      !Number.isSafeInteger(entry.id) ||
      typeof trackName !== "string" ||
      !trackName.trim()
    )
      continue;
    results.push({
      id: entry.id,
      trackName,
      artistName: typeof entry.artistName === "string" ? entry.artistName : "",
      albumName: typeof entry.albumName === "string" ? entry.albumName : "",
      duration:
        typeof entry.duration === "number" &&
        Number.isFinite(entry.duration) &&
        entry.duration >= 0
          ? entry.duration
          : null,
      instrumental: entry.instrumental === true,
      plainLyrics:
        typeof entry.plainLyrics === "string" ? entry.plainLyrics : null,
      syncedLyrics:
        typeof entry.syncedLyrics === "string" ? entry.syncedLyrics : null,
    });
  }
  if (body.length && !results.length)
    throw new Error(
      "LRCLIB returned no readable search results. Please try again.",
    );
  return results;
}
