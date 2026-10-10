import { fetch as nativeFetch } from "@tauri-apps/plugin-http";
import { createLyricsClient } from "./lyricsSearch";

export const lyricsClient = createLyricsClient(nativeFetch);
