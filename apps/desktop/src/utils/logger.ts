import { error as tauriError } from "@tauri-apps/plugin-log";

function formatValue(value: unknown): string {
  if (value instanceof Error) return value.stack ?? `${value.name}: ${value.message}`;
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

export function logError(context: string, error: unknown): void {
  const message = `${context}: ${formatValue(error)}`;
  void tauriError(message).catch(() => undefined);
}

export function installErrorLogging(): void {
  const originalError = console.error.bind(console);
  console.error = (...values: unknown[]) => {
    originalError(...values);
    logError("console.error", values.map(formatValue).join(" "));
  };

  window.addEventListener("error", (event) => {
    logError("window.error", event.error ?? `${event.message} at ${event.filename}:${event.lineno}:${event.colno}`);
  });

  window.addEventListener("unhandledrejection", (event) => {
    logError("unhandledrejection", event.reason);
  });
}
