import { parseLyricsSearchResponse } from "./lyricsSearchResponse.ts";

export function createLyricsClient(fetch: typeof globalThis.fetch) {
  return {
    async searchLyrics(
      { query }: { query: string },
      { signal }: { signal?: AbortSignal } = {},
    ) {
      const term = query.trim();
      if (!term) return [];
      const controller = new AbortController();
      const abort = () => controller.abort();
      if (signal?.aborted) abort();
      signal?.addEventListener("abort", abort, { once: true });
      const timeout = setTimeout(abort, 15_000);
      try {
        const url = new URL("https://lrclib.net/api/search");
        url.searchParams.set("q", term);
        let response = await fetch(url.toString(), {
          signal: controller.signal,
          headers: { "Lrclib-Client": "Audexis" },
        });
        for (
          let attempt = 0;
          attempt < 2 && [429, 502, 503, 504].includes(response.status);
          attempt++
        ) {
          await waitForRetry(500 * (attempt + 1), controller.signal);
          response = await fetch(url.toString(), {
            signal: controller.signal,
            headers: { "Lrclib-Client": "Audexis" },
          });
        }
        if (!response.ok) {
          const temporary = [429, 502, 503, 504].includes(response.status);
          throw new Error(
            temporary
              ? `LRCLIB's search service is temporarily unavailable (${response.status}). Please try again shortly.`
              : `LRCLIB search failed (${response.status}). Please try again.`,
          );
        }
        return parseLyricsSearchResponse(await response.json());
      } finally {
        clearTimeout(timeout);
        signal?.removeEventListener("abort", abort);
      }
    },
  };
}

function waitForRetry(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const abort = () => {
      clearTimeout(timer);
      signal.removeEventListener("abort", abort);
      reject(new Error("Lyrics search was cancelled or timed out."));
    };
    const timer = setTimeout(() => {
      signal.removeEventListener("abort", abort);
      resolve();
    }, ms);
    signal.addEventListener("abort", abort, { once: true });
    if (signal.aborted) abort();
  });
}
