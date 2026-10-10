import { clsx, type ClassValue } from "clsx";

import { twMerge } from "tailwind-merge";

export function parseShortcut(shortcut: string) {
  shortcut = shortcut.replace(
    "mod",
    navigator.platform.includes("Mac") ? "⌘" : "Ctrl",
  );

  shortcut = shortcut.replace("cmd", "⌘");
  shortcut = shortcut.replace("ctrl", "Ctrl");
  shortcut = shortcut.replace("alt", "Alt");
  shortcut = shortcut.replace("shift", "Shift");
  return shortcut;
}
export function shortcutAccelerator(shortcut: string) {
  shortcut = shortcut.replace("mod", "CmdOrCtrl");

  shortcut = shortcut.replace("shift", "Shift");
  return shortcut;
}

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

const illegalCharacters = /[\\/:*?"<>|]/;
const reservedFileNames = [
  "CON",
  "PRN",
  "AUX",
  "NUL",
  ...Array.from({ length: 9 }, (_, i) => `COM${i + 1}`),
  ...Array.from({ length: 9 }, (_, i) => `LPT${i + 1}`),
];

export function isValidFileName(
  name: string,
  ignoreMaxChar: boolean,
): { err: boolean; message?: string } {
  if (!name || name.trim().length === 0) {
    return { err: true, message: "File name cannot be empty." };
  }

  if (illegalCharacters.test(name)) {
    return { err: true, message: "File name contains illegal characters." };
  }

  if (name.endsWith(" ") || name.endsWith(".")) {
    return {
      err: true,
      message: "File name cannot end with a space or period.",
    };
  }

  const baseName = name.split(".")[0].toUpperCase();
  if (reservedFileNames.includes(baseName)) {
    return {
      err: true,
      message: `File name '${baseName}' is reserved by the system.`,
    };
  }

  if (name.length > 255 && !ignoreMaxChar) {
    return {
      err: true,
      message: "File name is too long (max 255 characters).",
    };
  }

  return { err: false };
}
