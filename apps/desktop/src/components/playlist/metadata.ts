export function titleFor(
  metadata: { key: string; value: string }[] | undefined,
  fallback: string,
) {
  return metadata?.find((item) => item.key === "title")?.value ?? fallback;
}

export function subtitleFor(
  metadata: { key: string; value: string }[] | undefined,
) {
  const artist = metadata?.find((item) => item.key === "artist")?.value ?? "";
  const album = metadata?.find((item) => item.key === "album")?.value ?? "";
  return [artist, album].filter(Boolean).join(" · ");
}
