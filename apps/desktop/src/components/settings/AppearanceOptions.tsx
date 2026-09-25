import { Check, Monitor, Moon, Sun } from "lucide-react";
import type { RowDensity, ThemePreference } from "../../hooks/useStore";
import { cn } from "../../utils";
import { ForwardedRef } from "react";
export type Appearance = {
  theme: ThemePreference;
  density: RowDensity;
  reduceMotion: boolean;
};
type ThemeOptions = {
  id: "light" | "dark" | "system";
  label: string;
  Icon: Function;
};
export function AppearanceOptions({
  value,
  onChange,
}: {
  value: Appearance;
  onChange: (value: Appearance) => void;
}) {
  const themeOptions: ThemeOptions[] = [
    { id: "light", label: "Light", Icon: Sun },
    { id: "dark", label: "Dark", Icon: Moon },
    { id: "system", label: "System", Icon: Monitor },
  ];
  return (
    <div className="space-y-7">
      <fieldset>
        <legend className="mb-3 text-sm font-semibold">Appearance</legend>
        <div className="grid grid-cols-3 gap-3">
          {themeOptions.map(({ id, label, Icon }) => (
            <button
              key={id}
              type="button"
              aria-pressed={value.theme === id}
              onClick={() => onChange({ ...value, theme: id })}
              className={cn(
                "rounded-xl border p-2.5 text-left transition-colors focus-visible:outline-2 focus-visible:outline-primary",
                value.theme === id
                  ? "border-primary bg-primary/5 ring-1 ring-primary"
                  : "border-border hover:bg-muted/30",
              )}
            >
              <div
                aria-hidden="true"
                className={cn(
                  "mb-3 flex h-16 gap-2 overflow-hidden rounded-lg border p-2",
                  id === "dark"
                    ? "border-white/10 bg-zinc-900"
                    : id === "system"
                      ? "border-black/10 bg-gradient-to-r from-stone-100 from-50% to-zinc-900 to-50%"
                      : "border-black/10 bg-stone-100",
                )}
              >
                <div className="w-4 rounded-sm bg-primary/25" />
                <div className="flex flex-1 flex-col gap-1.5 pt-1">
                  <div className="h-1.5 w-2/3 rounded bg-primary/60" />
                  <div className="h-1 rounded bg-stone-400/40" />
                  <div className="h-1 rounded bg-stone-400/40" />
                  <div className="h-1 w-3/4 rounded bg-stone-400/40" />
                </div>
              </div>
              <span className="flex items-center gap-1.5 text-xs font-medium">
                <Icon size={13} />
                {label}
                {value.theme === id && (
                  <Check size={13} className="ml-auto text-primary" />
                )}
              </span>
            </button>
          ))}
        </div>
        <p className="mt-2 text-xs text-muted-foreground">
          System follows your device’s light and dark appearance.
        </p>
      </fieldset>
      <fieldset>
        <legend className="mb-3 text-sm font-semibold">Song row spacing</legend>
        <div className="flex rounded-xl border border-border bg-muted/20 p-1">
          {(
            [
              { id: "compact", label: "Compact" },
              { id: "default", label: "Balanced" },
              { id: "comfort", label: "Comfortable" },
            ] as const
          ).map(({ id, label }) => (
            <button
              key={id}
              type="button"
              aria-pressed={value.density === id}
              onClick={() => onChange({ ...value, density: id })}
              className={cn(
                "flex-1 rounded-lg px-2 py-2 text-xs transition-colors",
                value.density === id
                  ? "bg-background font-semibold shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {label}
            </button>
          ))}
        </div>
      </fieldset>
      <label className="flex cursor-pointer items-start justify-between gap-6 rounded-xl border border-border p-4">
        <span>
          <span className="block text-sm font-semibold">Reduce motion</span>
          <span className="mt-1 block text-xs text-muted-foreground">
            Keep long song titles still in the Now Playing bar.
          </span>
        </span>
        <input
          type="checkbox"
          checked={value.reduceMotion}
          onChange={(event) =>
            onChange({ ...value, reduceMotion: event.target.checked })
          }
          className="mt-1 size-4 accent-[var(--primary)]"
        />
      </label>
    </div>
  );
}
