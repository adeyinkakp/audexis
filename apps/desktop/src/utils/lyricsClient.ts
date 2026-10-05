import { fetch as nativeFetch } from "@tauri-apps/plugin-http";
import { Client } from "lrclib-api";

export const lyricsClient = new Client({
  clientName: "Audexis",
  timeoutMs: 15_000,
  fetch: (input, init) => nativeFetch(input, init),
});
