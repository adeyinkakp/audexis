// @ts-check
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

import sitemap from "@astrojs/sitemap";

import node from "@astrojs/node";

// https://astro.build/config
export default defineConfig({
  site: "https://www.audexis.app",
  output: "server",

  vite: {
    plugins: [tailwindcss()],
    server: {
      allowedHosts: [
        "my-macbook-air.tail5c4e82.ts.net",
        "my-macbook-air.tail5c4e82.ts.net:443",
      ],
      fs: {
        allow: ["../../shared"],
      },
    },
  },

  integrations: [sitemap()],

  adapter: node({
    mode: "standalone",
  }),
});
