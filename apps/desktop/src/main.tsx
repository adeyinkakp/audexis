import React from "react";
import { useLibraryEvents } from "./hooks/useLibraryEvents";
import ReactDOM from "react-dom/client";

// import Titlebar from "./components/Titlebar";
import { StoreProvider } from "./hooks/useStore";
import { Toaster } from "react-hot-toast";
import "./styles/main.css";
import { Link, RouterProvider, createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MutationCache, QueryCache } from "@tanstack/react-query";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { installErrorLogging, logError } from "./utils/logger";

installErrorLogging();

export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error, query) => logError(`Query failed [${query.queryHash}]`, error),
  }),
  mutationCache: new MutationCache({
    onError: (error, _variables, _context, mutation) =>
      logError(`Mutation failed [${String(mutation.options.mutationKey ?? "unknown")}]`, error),
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
  defaultNotFoundComponent: () => {
    return (
      <div>
        <Link to="/">go home</Link>
      </div>
    );
  },
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

    <QueryClientProvider client={queryClient}>
      <StoreProvider>
        <LibraryEvents />
        <RouterProvider router={router} />
      </StoreProvider>
    </QueryClientProvider>
    </ErrorBoundary>
  </React.StrictMode>,
);
