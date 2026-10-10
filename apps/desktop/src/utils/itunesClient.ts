import { searchItunes } from "node-itunes-search/dist/mod.mjs";
import { fetch as nativeFetch } from "@tauri-apps/plugin-http";

export async function searchItunesSongs(term: string) {
  try {
    const response = await searchItunes({
      term,
      media: "music",
      entity: "song",
      limit: "50",
    });
    return response.results.filter(
      (match) => match.kind === "song" && typeof match.trackId === "number",
    );
  } catch (error) {
    if (error instanceof Response)
      throw new Error(`iTunes returned ${error.status}. Please try again.`);
    throw error;
  }
}

export type DownloadedArtwork = {
  mime: string;
  data_base64: string;
  picture_type: number;
  description: string;
};

export async function downloadArtwork(url: string): Promise<DownloadedArtwork> {
  const parsed = new URL(url);
  if (
    parsed.protocol !== "https:" ||
    !parsed.hostname.endsWith(".mzstatic.com") ||
    parsed.port ||
    parsed.username ||
    parsed.password
  ) {
    throw new Error("Unsupported artwork URL.");
  }
  const largeUrl = url.replace(/\/\d+x\d+bb\./, "/1000x1000bb.");
  let response = await nativeFetch(largeUrl, {
    signal: AbortSignal.timeout(20_000),
  });
  if (!response.ok && largeUrl !== url) {
    response = await nativeFetch(url, { signal: AbortSignal.timeout(20_000) });
  }
  if (!response.ok)
    throw new Error(`Artwork download failed (${response.status}).`);
  const blob = await response.blob();
  if (!blob.size || blob.size > 20 * 1024 * 1024)
    throw new Error("Artwork must be between 1 byte and 20 MB.");
  const bytes = new Uint8Array(await blob.arrayBuffer());
  // shoutout chat gtt
  const mime =
    bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff
      ? "image/jpeg"
      : [137, 80, 78, 71, 13, 10, 26, 10].every(
            (byte, index) => bytes[index] === byte,
          )
        ? "image/png"
        : undefined;
  if (!mime) throw new Error("Artwork must be a JPEG or PNG image.");
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 8192) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
  }
  return {
    mime,
    data_base64: btoa(binary),
    picture_type: 3,
    description: "Front cover",
  };
}
