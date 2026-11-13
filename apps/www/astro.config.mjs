// @ts-check
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

import sitemap from "@astrojs/sitemap";

import node from "@astrojs/node";
import { config as loadEnv } from "dotenv";

// https://astro.build/config
export default defineConfig({
  site: "https://www.audexis.app",
  output: "server",
  security: {
    allowedDomains: [
      { hostname: "www.audexis.app", protocol: "https" },
      { hostname: "audexis.app", protocol: "https" },
    ],
  },

  vite: {
    plugins: [tailwindcss()],
    server: {
      fs: {
        allow: ["../../shared"],
      },
    },
  },

  integrations: [
    sitemap(),
    {
      name: "audexis-development-origins",
      hooks: {
        "astro:config:setup": ({ command, updateConfig }) => {
          if (command !== "dev") return;
          loadEnv({ quiet: true });
          const origins = [
            "http://localhost:4321",
            "http://127.0.0.1:4321",
            process.env.BETTER_AUTH_URL?.trim(),
            ...(process.env.AUDEXIS_DEV_AUTH_ORIGINS ?? "").split(","),
          ]
            .map((value) => value?.trim())
            .filter(Boolean)
            .map((value) => new URL(value));
          updateConfig({
            security: {
              allowedDomains: origins.map((origin) => ({
                hostname: origin.hostname,
                protocol: origin.protocol.slice(0, -1),
              })),
            },
            vite: {
              server: {
                allowedHosts: [
                  ...new Set(origins.map((origin) => origin.hostname)),
                ],
              },
            },
          });
        },
      },
    },
  ],

  adapter: node({
    mode: "standalone",
  }),
});
