import { Component, type ErrorInfo, type ReactNode } from "react";
import { logError } from "../utils/logger";

export class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    logError("React render error", `${error.stack ?? error.message}\n${info.componentStack}`);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex h-screen items-center justify-center bg-background p-8 text-foreground">
          <div className="max-w-lg rounded-2xl border border-border bg-popover p-8 text-center">
            <h1 className="text-xl font-semibold">Audexis ran into a problem</h1>
            <p className="mt-3 text-sm text-muted-foreground">
              The error was saved to the application log.
            </p>
            <button
              type="button"
              onClick={() => window.location.reload()}
              className="mt-6 rounded-full bg-primary px-5 py-2 text-sm font-semibold text-primary-foreground"
            >
              Reload
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
