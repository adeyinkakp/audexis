import { useState } from "react";
import type { ItunesResult } from "node-itunes-search";
import OnlineLookup from "./OnlineLookup";
import {
  searchItunesSongs,
  downloadArtwork,
  type DownloadedArtwork,
} from "../../utils/itunesClient";
import { artworkUrl, metadataFromItunes } from "../../utils/itunesMetadata";
import { formatDuration } from "../../utils/duration";

export default function ItunesLookup({
  title,
  artist,
  disabled,
  onMetadata,
  onClose,
  onBusyChange,
}: {
  title: string;
  artist: string;
  disabled: boolean;
  onMetadata: (match: ItunesResult, image?: DownloadedArtwork) => Promise<void>;
  onClose: () => void;
  onBusyChange?: (busy: boolean) => void;
}) {
  const [downloading, setDownloading] = useState(false);
  const [includeArtwork, setIncludeArtwork] = useState(true);
  const [error, setError] = useState("");
  return (
    <OnlineLookup
      heading="Find metadata on iTunes"
      description="Search by song and artist. Choose Use to save the result directly to the selected files."
      initialSearch={[title, artist].filter(Boolean).join(" ")}
      provider="iTunes"
      disabled={disabled || downloading}
      search={searchItunesSongs}
      getId={(match) => match.trackId!}
      onClose={onClose}
      renderResult={(match) => (
        <>
          <span className="block text-sm font-medium">{match.trackName}</span>
          <span className="mt-1 block text-xs text-muted-foreground">
            {match.artistName} · {match.collectionName || "Unknown album"}
            {match.trackTimeMillis != null &&
              ` · ${formatDuration(match.trackTimeMillis)}`}
          </span>
        </>
      )}
      renderPreview={(match) => {
        const url = artworkUrl(match);
        return (
          <>
            {url && (
              <img
                src={url}
                alt={`${match.collectionName || match.trackName} cover`}
                className="h-32 w-32 rounded-lg object-contain"
              />
            )}
            <dl className="grid grid-cols-2 gap-1 text-xs">
              {Object.entries(metadataFromItunes(match)).map(([key, value]) => (
                <div key={key}>
                  <dt className="text-muted-foreground">
                    {
                      {
                        title: "Title",
                        artist: "Artist",
                        album: "Album",
                        genre: "Genre",
                        year: "Year",
                        trackNumber: "Track number",
                        discnumber: "Disc number",
                      }[key]
                    }
                  </dt>
                  <dd>{value}</dd>
                </div>
              ))}
            </dl>

            {url && (
              <label className="flex items-center gap-2 text-xs">
                <input
                  type="checkbox"
                  checked={includeArtwork}
                  disabled={disabled || downloading}
                  onChange={(event) => setIncludeArtwork(event.target.checked)}
                />
                Include artwork as the front cover
              </label>
            )}
            {
              <button
                type="button"
                disabled={
                  disabled ||
                  downloading ||
                  !Object.keys(metadataFromItunes(match)).length
                }
                onClick={async () => {
                  setError("");
                  setDownloading(true);
                  onBusyChange?.(true);
                  try {
                    const image =
                      includeArtwork && url
                        ? await downloadArtwork(url)
                        : undefined;
                    await onMetadata(match, image);
                    onClose();
                  } catch (error) {
                    setError(
                      error instanceof Error ? error.message : String(error),
                    );
                  } finally {
                    setDownloading(false);
                    onBusyChange?.(false);
                  }
                }}
                className="h-9 rounded-lg bg-primary px-3 text-sm text-primary-foreground disabled:opacity-40"
              >
                {downloading ? "Saving…" : "Use metadata"}
              </button>
            }
            {error && (
              <p role="alert" className="text-xs text-destructive">
                {error}
              </p>
            )}
          </>
        );
      }}
    />
  );
}
