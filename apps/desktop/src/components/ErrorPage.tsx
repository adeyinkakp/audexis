import { AlertTriangle, ArrowLeft, Disc3, Home, RefreshCw } from "lucide-react";

type ErrorPageProps = {
  kind: "not-found" | "error" | "fatal";
  error?: unknown;
  onRetry?: () => void;
};

const content = {
  "not-found": {
    mainMessage: "404 - not found",
    title: "Page could not be found",
    description:
      "The page may have moved, been removed, or never made it into your library.",
  },
  error: {
    mainMessage: "Playback interrupted",
    title: "Unexpected Error",
    description: "Audexis could not load this page.",
  },
  fatal: {
    mainMessage: "Application error",
    title: "Audexis ran into a problem.",
    description:
      "The error was saved to the application log. Reload the app to get back to your music.",
  },
} as const;

function getErrorMessage(error: unknown) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return null;
}

export function ErrorPage({ kind, error, onRetry }: ErrorPageProps) {
  const copy = content[kind];
  const errorMessage = getErrorMessage(error);
  const isNotFound = kind === "not-found";

  return (
    <main className="relative mt-14 flex min-h-[calc(100dvh-3.5rem)] items-center justify-center overflow-hidden bg-background px-6 py-12 text-foreground">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 opacity-70"
        style={{
          background:
            "radial-gradient(circle at 18% 22%, color-mix(in srgb, var(--primary) 14%, transparent), transparent 30%), radial-gradient(circle at 82% 78%, color-mix(in srgb, var(--primary) 8%, transparent), transparent 28%)",
        }}
      />

      <section className="relative w-full max-w-2xl overflow-hidden rounded-[2rem] border border-border/70 bg-popover/85 p-7 shadow-2xl shadow-black/10 backdrop-blur-xl sm:p-10">
        <div className="pointer-events-none absolute -right-16 -top-20 size-64 rounded-full border border-border/40" />
        <div className="pointer-events-none absolute -right-8 -top-12 size-48 rounded-full border border-border/50" />

        <div className="relative flex flex-col gap-8 sm:flex-row sm:items-center">
          <div
            aria-hidden="true"
            className="relative flex size-28 shrink-0 items-center justify-center rounded-full border border-border bg-background shadow-inner sm:size-36"
          >
            <div className="absolute inset-3 rounded-full border border-border/70" />
            <div className="absolute inset-7 rounded-full border border-border/60" />
            <Disc3
              className="size-14 text-primary sm:size-16"
              strokeWidth={1.25}
            />
            <span className="absolute bottom-1 right-1 flex size-9 items-center justify-center rounded-full border-4 border-popover bg-destructive text-destructive-foreground sm:size-10">
              <AlertTriangle size={16} />
            </span>
          </div>

          <div className="min-w-0 flex-1">
            <p className="mb-3 text-xs font-semibold uppercase tracking-[0.18em] text-primary">
              {copy.mainMessage}
            </p>
            <h1 className="max-w-xl text-2xl font-bold tracking-tight sm:text-3xl">
              {copy.title}
            </h1>
            <p className="mt-3 max-w-lg text-sm leading-6 text-muted-foreground">
              {copy.description}
            </p>

            <div className="mt-7 flex flex-wrap gap-3">
              {!isNotFound && onRetry && (
                <button
                  type="button"
                  onClick={onRetry}
                  className="inline-flex items-center gap-2 rounded-full bg-primary px-5 py-2.5 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                >
                  <RefreshCw size={16} />
                  {kind === "fatal" ? "Reload Audexis" : "Try again"}
                </button>
              )}
              <button
                type="button"
                onClick={() => window.location.assign("/")}
                className="inline-flex items-center gap-2 rounded-full border border-border bg-background/60 px-5 py-2.5 text-sm font-semibold transition-colors hover:bg-muted"
              >
                <Home size={16} />
                Go home
              </button>
              {isNotFound && (
                <button
                  type="button"
                  onClick={() => window.history.back()}
                  className="inline-flex items-center gap-2 rounded-full px-4 py-2.5 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
                >
                  <ArrowLeft size={16} />
                  Go back
                </button>
              )}
            </div>
          </div>
        </div>

        {errorMessage && (
          <details className="relative mt-8 border-t border-border/60 pt-5">
            <summary className="cursor-pointer select-none text-xs font-medium text-muted-foreground transition-colors hover:text-foreground">
              Show technical details
            </summary>
            <pre className="mt-3 max-h-32 overflow-auto whitespace-pre-wrap break-words rounded-xl bg-background/70 p-4 text-xs leading-5 text-muted-foreground">
              {errorMessage}
            </pre>
          </details>
        )}
      </section>
    </main>
  );
}
