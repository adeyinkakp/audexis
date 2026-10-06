import React from "react";
import { useLibraryEvents } from "./hooks/useLibraryEvents";
import ReactDOM from "react-dom/client";

import { StoreProvider } from "./hooks/StoreProvider";
import { Toaster } from "react-hot-toast";
import "./styles/main.css";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MutationCache, QueryCache } from "@tanstack/react-query";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ErrorPage } from "./components/ErrorPage";
import { StartupValidate } from "./components/StartupValidate";
import { installErrorLogging, logError } from "./utils/logger";

import MiniPlayer from "./components/MiniPlayer";

const isMiniPlayer = new URLSearchParams(window.location.search).has("mini-player");

installErrorLogging();

export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error, query) =>
      logError(`Query failed [${query.queryHash}]`, error),
  }),
  mutationCache: new MutationCache({
    onError: (error, _variables, _context, mutation) =>
      logError(
        `Mutation failed [${String(mutation.options.mutationKey ?? "unknown")}]`,
        error,
      ),
  }),
  defaultOptions: {
    queries: {
      staleTime: Infinity,
      gcTime: Infinity,
    },
  },
});

const router = createRouter({
  routeTree,
  context: { queryClient },
  defaultPreload: "intent",
  scrollRestoration: true,
  defaultErrorComponent: ({ error, reset }) => (
    <ErrorPage kind="error" error={error} onRetry={reset} />
  ),
  defaultNotFoundComponent: () => <ErrorPage kind="not-found" />,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

function LibraryEvents() {
  useLibraryEvents();
  return null;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <Toaster
        position="top-right"
        containerStyle={{
          marginTop: "64px",
          zIndex: 20000,
        }}
        toastOptions={{
          className: "!bg-background !text-foreground !border !border-border",
          style: {
            background: "var(--background)",
            color: "var(--foreground)",
            border: "1px solid var(--border)",
          },
        }}
      />

      <StartupValidate>
        <QueryClientProvider client={queryClient}>
          {isMiniPlayer ? (
            <>
              <LibraryEvents />
              <MiniPlayer />
            </>
          ) : (
            <StoreProvider>
              <LibraryEvents />
              <RouterProvider router={router} />
            </StoreProvider>
          )}
        </QueryClientProvider>
      </StartupValidate>
    </ErrorBoundary>
  </React.StrictMode>,
);
