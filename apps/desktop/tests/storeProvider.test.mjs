import test from "node:test";
import assert from "node:assert/strict";
import React from "react";
import { renderToString } from "react-dom/server";
import { createServer } from "vite";

test("store consumers share the provider context across module reloads", async () => {
  const previousWindow = globalThis.window;
  const server = await createServer({
    server: { middlewareMode: true, hmr: false, ws: false },
    appType: "custom",
  });
  try {
    const { StoreProvider } = await server.ssrLoadModule(
      "/src/hooks/StoreProvider.tsx",
    );
    let { useStore } = await server.ssrLoadModule("/src/hooks/useStore.tsx");
    globalThis.window = { matchMedia: () => ({ matches: false }) };
    function Consumer() {
      return React.createElement("span", null, useStore().preferences.density);
    }
    assert.equal(
      renderToString(
        React.createElement(StoreProvider, null, React.createElement(Consumer)),
      ),
      '<div class="h-full w-full"><span>default</span></div>',
    );
    const module = await server.moduleGraph.getModuleByUrl(
      "/src/hooks/useStore.tsx",
    );
    server.moduleGraph.invalidateModule(module);
    ({ useStore } = await server.ssrLoadModule("/src/hooks/useStore.tsx"));
    assert.equal(
      renderToString(
        React.createElement(StoreProvider, null, React.createElement(Consumer)),
      ),
      '<div class="h-full w-full"><span>default</span></div>',
    );
    assert.throws(
      () => renderToString(React.createElement(Consumer)),
      /StoreProvider/,
    );
  } finally {
    await server.close();
    globalThis.window = previousWindow;
  }
});
