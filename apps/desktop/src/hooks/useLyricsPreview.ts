import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";

type Status = {
  positionMs: number;
  durationMs: number;
  playing: boolean;
  error: string | null;
};
type Action = "open" | "play" | "pause" | "seek" | "status" | "close";
const empty: Status = {
  positionMs: 0,
  durationMs: 0,
  playing: false,
  error: null,
};

export function useLyricsPreview(fileId: number, enabled: boolean) {
  const [status, setStatus] = useState(empty);
  const [attempt, setAttempt] = useState(0);
  const [ready, setReady] = useState(false);
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const session = useRef<string | null>(null);
  const position = useRef(0);
  const positionFile = useRef(fileId);

  const send = (sessionId: string, action: Action, positionMs?: number) => {
    const next = queue.current.then(() =>
      invoke<Status>("lyrics_preview", {
        sessionId,
        action,
        fileId,
        positionMs,
      }),
    );
    queue.current = next.catch(() => {});
    return next;
  };

  const apply = (result: Status) => {
    position.current = result.positionMs;
    setStatus(result);
  };

  useEffect(() => {
    setReady(false);
    if (!enabled) {
      setStatus((current) => ({ ...current, playing: false }));
      return;
    }
    if (positionFile.current !== fileId) {
      position.current = 0;
      positionFile.current = fileId;
      setStatus(empty);
    } else {
      setStatus((current) => ({ ...current, error: null }));
    }
    const id = crypto.randomUUID();
    session.current = id;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const fail = (error: unknown) => {
      if (!disposed) {
        setReady(false);
        setStatus((current) => ({
          ...current,
          playing: false,
          error: String(error),
        }));
      }
    };
    const poll = async () => {
      try {
        const result = await send(id, "status");
        if (!disposed) apply(result);
      } catch (error) {
        fail(error);
        return;
      }
      if (!disposed) timer = setTimeout(() => void poll(), 50);
    };
    void (async () => {
      try {
        let result = await send(id, "open");
        if (disposed) return;
        if (position.current > 0)
          result = await send(id, "seek", position.current);
        if (disposed) return;
        apply(result);
        setReady(true);
        void poll();
      } catch (error) {
        fail(error);
      }
    })();
    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      session.current = null;
      void send(id, "close").catch(() => {});
    };
  }, [fileId, enabled, attempt]);

  const control = async (action: "play" | "pause" | "seek", time?: number) => {
    const id = session.current;
    if (!id || !ready) return;
    try {
      const result = await send(id, action, time);
      if (session.current === id) apply(result);
    } catch (error) {
      if (session.current === id)
        setStatus((current) => ({
          ...current,
          playing: false,
          error: String(error),
        }));
    }
  };

  return {
    position: status.positionMs,
    duration: status.durationMs,
    playing: status.playing,
    error: status.error,
    ready,
    retry: () => setAttempt((current) => current + 1),
    play: () => control("play"),
    pause: () => control("pause"),
    seek: (time: number) => control("seek", Math.round(time)),
    stamp: () => position.current,
  };
}
