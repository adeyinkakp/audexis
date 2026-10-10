import { songSelectionMenu } from "./songSelectionMenu";

export function selectAllForTarget(target: HTMLElement | null) {
  if (target?.matches("input, textarea")) {
    target.focus({ preventScroll: true });
    (target as HTMLInputElement | HTMLTextAreaElement).select();
  } else if (target?.isContentEditable) {
    target.focus({ preventScroll: true });
    document.execCommand("selectAll");
  } else {
    songSelectionMenu.selectAll();
  }
}

export async function editTextForTarget(
  command: "cut" | "copy" | "paste",
  target: HTMLElement | null,
) {
  if (!target) return;
  target.focus({ preventScroll: true });
  if (command === "copy" || command === "cut") {
    document.execCommand(command);
  } else {
    const text = await navigator.clipboard.readText();
    if (!target.isConnected) return;
    target.focus({ preventScroll: true });

    document.execCommand("insertText", false, text);
  }
}
