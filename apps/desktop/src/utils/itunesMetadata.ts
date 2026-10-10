import type { ItunesResult } from "node-itunes-search";

export function metadataFromItunes(match: ItunesResult) {
  const values = {
    title: match.trackName,
    artist: match.artistName,
    album: match.collectionName,
    genre: match.primaryGenreName,
    year: match.releaseDate?.match(/^\d{4}/)?.[0],
    trackNumber:
      match.trackNumber && match.trackNumber > 0
        ? String(match.trackNumber)
        : undefined,
    discnumber:
      match.discNumber && match.discNumber > 0
        ? String(match.discNumber)
        : undefined,
  };
  return Object.fromEntries(
    Object.entries(values).filter(([, value]) => value?.trim()),
  ) as Partial<Record<keyof typeof values, string>>;
}

export function artworkUrl(match: ItunesResult): string | undefined {
  const raw = match.artworkUrl100 ?? match.artworkUrl60 ?? match.artworkUrl30;
  if (!raw) return;
  try {
    const url = new URL(raw);
    if (
      url.protocol !== "https:" ||
      !url.hostname.endsWith(".mzstatic.com") ||
      url.port ||
      url.username ||
      url.password
    )
      return;
    return url.href;
  } catch {
    return;
  }
}
