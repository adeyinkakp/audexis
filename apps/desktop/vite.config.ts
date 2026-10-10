import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
import { tanstackRouter } from "@tanstack/router-plugin/vite";

// https://vite.dev/config/
// @ts-expect-error
export default defineConfig(async () => ({
  optimizeDeps: { exclude: ["node-itunes-search"] },
  plugins: [
    {
      name: "itunes-native-fetch",
      enforce: "pre",
      transform(code, id) {
        if (!/node-itunes-search\/dist\/mod\.(?:m)?js$/.test(id)) return;
        return {
          code:
            'import { fetch as itunesFetch } from "@tauri-apps/plugin-http";\nconst fetch = (url) => itunesFetch(url, { signal: AbortSignal.timeout(15000) });\n' +
            code,
          map: null,
        };
      },
    },
    tanstackRouter({
      target: "react",
      autoCodeSplitting: true,
    }),
    react(),
    tailwindcss(),
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
