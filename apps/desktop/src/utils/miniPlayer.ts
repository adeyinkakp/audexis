import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

let opening: Promise<void> | undefined;

export function openMiniPlayer(): Promise<void> {
  if (opening) return opening;
  opening = (async () => {
    const existing = await WebviewWindow.getByLabel("mini-player");
    if (existing) {
      await existing.show();
      await existing.setFocus();
      return;
    }
    const window = new WebviewWindow("mini-player", {
      url: "index.html?mini-player",
      title: "Audexis Mini Player",
      width: 380,
      height: 260,
      resizable: false,
      maximizable: false,
      decorations: false,
      center: true,
      visible: false,
    });
    await new Promise<void>((resolve, reject) => {
      void window.once("tauri://created", () => resolve()).catch(reject);
      void window.once("tauri://error", (event) => reject(new Error(String(event.payload)))).catch(reject);
    });
  })().finally(() => { opening = undefined; });
  return opening;
}
