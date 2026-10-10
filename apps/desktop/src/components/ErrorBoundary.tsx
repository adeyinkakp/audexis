import { Component, type ErrorInfo, type ReactNode } from "react";
import { logError } from "../utils/logger";
import { ErrorPage } from "./ErrorPage";

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
        <ErrorPage
          kind="fatal"
          error={this.state.error}
          onRetry={() => window.location.reload()}
        />
      );
    }
    return this.props.children;
  }
}
