import { useState } from "react";
import { lyricsClient } from "../../utils/lyricsClient";
import { formatDuration } from "../../utils/duration";
import OnlineLookup from "./OnlineLookup";

export default function LyricsLookup({
  title,
  artist,
  synchronized,
  hasDraft,
  disabled,
  onApply,
  onClose,
}: {
  title: string;
  artist: string;
  synchronized: boolean;
  hasDraft: boolean;
  disabled: boolean;
  onApply: (lyrics: string) => Promise<void>;
  onClose: () => void;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const type = synchronized ? "synchronized" : "plain";
  return (
    <OnlineLookup
      heading="Find lyrics on LRCLIB"
      description={`Search by song and artist, then choose ${type} lyrics to save to the selected files.`}
      initialSearch={[title, artist].filter(Boolean).join(" ")}
      provider="LRCLIB"
      disabled={disabled || saving}
      search={(query, signal) =>
        lyricsClient.searchLyrics({ query }, { signal })
      }
      getId={(match) => match.id}
      onClose={onClose}
      renderResult={(match) => (
        <>
          <span className="block text-sm font-medium">{match.trackName}</span>
          <span className="mt-1 block text-xs text-muted-foreground">
            {match.artistName} · {match.albumName || "Unknown album"} ·{" "}
            {formatDuration(
              match.duration === null ? null : match.duration * 1000,
            )}
          </span>
          <span className="mt-2 block text-[11px] text-muted-foreground">
            {match.instrumental
              ? "Instrumental"
              : [
                  match.plainLyrics?.trim() && "Plain",
                  match.syncedLyrics?.trim() && "Synchronized",
                ]
                  .filter(Boolean)
                  .join(" · ") || "No lyrics"}
          </span>
        </>
      )}
      renderPreview={(selected) => {
        const text =
          (synchronized ? selected.syncedLyrics : selected.plainLyrics) ?? "";
        return (
          <>
            <p className="text-xs font-medium">
              {synchronized
                ? "Synchronized lyrics preview"
                : "Plain lyrics preview"}
            </p>
            {text.trim() ? (
              <pre className="max-h-44 overflow-auto whitespace-pre-wrap wrap-break-word rounded-lg bg-background p-3 font-sans text-xs leading-6">
                {text}
              </pre>
            ) : (
              <p className="text-sm text-muted-foreground">
                {selected.instrumental
                  ? "This recording is marked as instrumental."
                  : `This match has no ${type} lyrics. Choose another result.`}
              </p>
            )}
            <p className="text-xs text-muted-foreground">
              {hasDraft ? "This replaces the text in this tab. " : ""}Use lyrics
              saves immediately to the selected files.
            </p>
            <button
              type="button"
              disabled={!text.trim() || disabled || saving}
              onClick={async () => {
                setError("");
                setSaving(true);
                try {
                  await onApply(text);
                  onClose();
                } catch (error) {
                  setError(
                    error instanceof Error ? error.message : String(error),
                  );
                } finally {
                  setSaving(false);
                }
              }}
              className="h-9 rounded-lg bg-primary px-3 text-sm text-primary-foreground disabled:opacity-40"
            >
              {saving ? "Saving…" : `Use ${type} lyrics`}
            </button>
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
