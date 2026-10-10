import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { useMediaFiles } from "./useMediaFiles";

export type CurrentMediaInfo = {
  id: number;
  current_position: number;
  duration: number;
  paused: boolean;
};

type PartialMediaInfo = {
  id: number;
  duration: number;
};

function metadataValue(
  metadata: { key: string; value: string }[] | undefined,
  key: string,
) {
  return metadata?.find((item) => item.key === key)?.value ?? "";
}

export function useNowPlayingState() {
  const [paused, setPaused] = useState(false);
  const [position, setPosition] = useState(0);
  const [duration, setDuration] = useState(0);
  const [currentTrackId, setCurrentTrackId] = useState(0);
  const { data } = useMediaFiles(currentTrackId ? [currentTrackId] : []);
  const artwork = useQuery({
    queryKey: ["libraryArtwork", currentTrackId],
    queryFn: () =>
      invoke<string | null>("get_artwork", { fileId: currentTrackId }),
    enabled: currentTrackId > 0,
  });
  const lastControlsTrackId = useRef(0);
  const lastControlsTitle = useRef("");
  const lastControlsDuration = useRef(0);

  useEffect(() => {
    let disposed = false;
    let cleanupUpdate: (() => void) | undefined;
    let cleanupStart: (() => void) | undefined;
    let cleanupQueueDone: (() => void) | undefined;

    void listen<CurrentMediaInfo>("player-update", (event) => {
      const hasActiveTrack =
        event.payload.id !== 0 && event.payload.duration !== 0;

      if (!hasActiveTrack) {
        setPaused(false);
        setPosition(0);
        setDuration(0);
        setCurrentTrackId(0);
        return;
      }

      setPaused(event.payload.paused);
      setPosition(event.payload.current_position / 1000);
      setDuration(event.payload.duration / 1000);
      setCurrentTrackId(event.payload.id);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else cleanupUpdate = unlisten;
    });

    void listen<PartialMediaInfo>("playback-media-start", (event) => {
      setCurrentTrackId(event.payload.id);
      setDuration(event.payload.duration / 1000);
      setPosition(0);
      setPaused(false);
      lastControlsTrackId.current = event.payload.id;
      lastControlsDuration.current = event.payload.duration;
    }).then((unlisten) => {
      if (disposed) unlisten();
      else cleanupStart = unlisten;
    });

    void listen("playback-queue-done", () => {
      setPaused(false);
      setPosition(0);
      setDuration(0);
      setCurrentTrackId(0);
      lastControlsTrackId.current = 0;
      lastControlsTitle.current = "";
      lastControlsDuration.current = 0;
    }).then((unlisten) => {
      if (disposed) unlisten();
      else cleanupQueueDone = unlisten;
    });

    return () => {
      disposed = true;
      cleanupUpdate?.();
      cleanupStart?.();
      cleanupQueueDone?.();
    };
  }, []);

  const currentFile = currentTrackId
    ? data?.files.find((file) => file.id === currentTrackId)
    : undefined;
  const currentMetadata = currentTrackId
    ? data?.metadata.filter((item) => item.file_id === currentTrackId)
    : undefined;

  const currentTitle =
    metadataValue(currentMetadata, "title") || currentFile?.file_name || "";
  const effectiveDurationMs =
    duration > 0
      ? Math.round(duration * 1000)
      : (currentFile?.duration_ms ?? 0);

  useEffect(() => {
    if (!currentTrackId || !currentTitle || !effectiveDurationMs) return;

    if (
      lastControlsTrackId.current === currentTrackId &&
      lastControlsTitle.current === currentTitle &&
      lastControlsDuration.current === effectiveDurationMs
    ) {
      return;
    }

    lastControlsTrackId.current = currentTrackId;
    lastControlsTitle.current = currentTitle;
    lastControlsDuration.current = effectiveDurationMs;

    void invoke("update_media_controls", {
      fileId: currentTrackId,
    });
  }, [currentTitle, currentTrackId, effectiveDurationMs]);

  const song = useMemo(() => {
    if (!currentFile) return null;

    return {
      id: currentFile.id,
      path: currentFile.path,
      title: currentTitle || currentFile.file_name,
      album: metadataValue(currentMetadata, "album"),
      artist: metadataValue(currentMetadata, "artist"),
      genre: metadataValue(currentMetadata, "genre"),
      artwork_url: artwork.data ?? null,
      duration_ms: currentFile.duration_ms,
    };
  }, [artwork.data, currentFile, currentMetadata, currentTitle]);

  return {
    song,
    currentTrackId,
    paused,
    position,
    duration,
    setPosition,
    setPaused,
  };
}
