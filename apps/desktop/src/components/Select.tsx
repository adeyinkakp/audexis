"use client";

import * as React from "react";
import { Check, ChevronDown } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./Dropdown";
import { cn } from "../utils";

export type SelectOption<V extends string = string> = {
  value: V;
  label: string;
  disabled?: boolean;
  icon?: React.ReactNode;
};

export type SelectSize = "sm" | "md" | "lg";

export interface SelectProps<V extends string = string> {
  value?: V;
  onChange?: (value: V) => void;
  options: SelectOption<V>[];
  placeholder?: string;
  className?: string;
  contentClassName?: string;
  disabled?: boolean;
  fullWidth?: boolean;
  size?: SelectSize;
  align?: "start" | "center" | "end";
  side?: "top" | "bottom" | "left" | "right";
  ariaLabel?: string;
}

function sizeClasses(size: SelectSize = "md") {
  switch (size) {
    case "sm":
      return "h-7 text-sm px-2";
    case "lg":
      return "h-10 text-base px-3";
    case "md":
    default:
      return "h-9 text-sm px-3";
  }
}

export function Select<V extends string = string>({
  value,
  onChange,
  options,
  placeholder = "Select…",
  className,
  contentClassName,
  disabled,
  fullWidth,
  size = "sm",
  align = "start",
  side = "bottom",
  ariaLabel,
}: SelectProps<V>) {
  const [open, setOpen] = React.useState(false);
  const typeaheadRef = React.useRef<{ text: string; timer: number | null }>({
    text: "",
    timer: null,
  });
  const selected = options.find((o) => o.value === value);

  const getIndex = React.useCallback(
    (val: V | undefined) => options.findIndex((o) => o.value === val),
    [options],
  );

  const nextEnabledIndex = React.useCallback(
    (start: number, delta: 1 | -1) => {
      if (options.length === 0) return -1;
      let i = start;
      for (let step = 0; step < options.length; step++) {
        i = (i + delta + options.length) % options.length;
        if (!options[i]?.disabled) return i;
      }
      return start;
    },
    [options],
  );

  const commitIndex = React.useCallback(
    (idx: number) => {
      if (idx < 0 || idx >= options.length) return;
      const opt = options[idx];
      if (opt?.disabled) return;
      onChange?.(opt.value);
    },
    [onChange, options],
  );

  const onTriggerKeyDown = React.useCallback(
    (e: React.KeyboardEvent<HTMLButtonElement>) => {
      if (disabled) return;
      const currentIndex = getIndex(value);
      switch (e.key) {
        case "ArrowDown": {
          if (!open) {
            e.preventDefault();
            const next = nextEnabledIndex(
              currentIndex >= 0 ? currentIndex : -1,
              1,
            );
            if (next !== -1) commitIndex(next);
          }
          break;
        }
        case "ArrowUp": {
          if (!open) {
            e.preventDefault();
            const prev = nextEnabledIndex(
              currentIndex >= 0 ? currentIndex : 0,
              -1,
            );
            if (prev !== -1) commitIndex(prev);
          }
          break;
        }
        case "Enter":
        case " ": {
          e.preventDefault();
          setOpen((o) => !o);
          break;
        }
        case "Escape": {
          if (open) {
            e.preventDefault();
            setOpen(false);
          }
          break;
        }
        default: {
          if (
            !open &&
            e.key.length === 1 &&
            !e.metaKey &&
            !e.ctrlKey &&
            !e.altKey
          ) {
            const ch = e.key.toLowerCase();
            const ref = typeaheadRef.current;
            ref.text = (ref.text + ch).slice(-32);
            const foundIdx = options.findIndex((o) =>
              (o.label ?? "").toLowerCase().startsWith(ref.text),
            );
            if (foundIdx !== -1) commitIndex(foundIdx);
            if (ref.timer) window.clearTimeout(ref.timer);
            ref.timer = window.setTimeout(() => {
              ref.text = "";
              ref.timer = null;
            }, 600);
          }
        }
      }
    },
    [disabled, getIndex, value, open, nextEnabledIndex, commitIndex],
  );

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label={ariaLabel}
          disabled={disabled}
          data-state={open ? "open" : "closed"}
          onKeyDown={onTriggerKeyDown}
          className={cn(
            "group inline-flex items-center justify-between rounded border border-border bg-muted/20 text-foreground focus:outline-none focus:ring-2 focus:ring-primary/40 disabled:opacity-60 disabled:cursor-not-allowed",
            sizeClasses(size),
            fullWidth ? "w-full" : "",
            className,
          )}
        >
          <span
            className={cn("truncate text-left", fullWidth ? "w-full" : "")}
            title={selected?.label ?? placeholder}
          >
            {selected ? (
              selected.label
            ) : (
              <span className="text-muted-foreground">{placeholder}</span>
            )}
          </span>
          <ChevronDown className="ml-2 size-4 text-muted-foreground transition-transform group-data-[state=open]:rotate-180" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        className={cn("min-w-48 z-99999999999 p-1 max-h-56", contentClassName)}
        align={align}
        side={side}
        sideOffset={6}
      >
        {options.map((opt) => {
          const isSelected = opt.value === value;
          return (
            <DropdownMenuItem
              key={opt.value}
              disabled={opt.disabled}
              onSelect={(e) => {
                e.preventDefault();
                if (opt.disabled) return;
                onChange?.(opt.value);
                setOpen(false);
              }}
              className={cn(
                "flex items-center gap-2 px-2 py-1.5 rounded text-sm",
                isSelected && "bg-primary/10 text-primary",
              )}
            >
              <span className={cn("inline-flex items-center gap-2")}>
                {isSelected ? (
                  <Check className="size-4" />
                ) : (
                  <span className="w-4" />
                )}
                {opt.icon}
                <span className="truncate">{opt.label}</span>
              </span>
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export default Select;
