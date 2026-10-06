import { LogicalPosition } from "@tauri-apps/api/dpi";
import { emitTo, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export async function showMainPlayerWindow() {
  let main = await WebviewWindow.getByLabel("main");
  if (!main) {
    let ready!: () => void;
    let failed!: (error: Error) => void;
    const mounted = new Promise<void>((resolve, reject) => {
      ready = resolve;
      failed = reject;
    });
    const unlisten = await listen("main-player-ready", () => ready());
    const timeout = setTimeout(
      () =>
        failed(
          new Error("The main player took too long to open. Please try again."),
        ),
      20000,
    );
    let unlistenError: (() => void) | undefined;
    try {
      main = new WebviewWindow("main", {
        url: "index.html?expanded-player",
        title: "Audexis",
        width: 1100,
        height: 800,
        minWidth: 1000,
        minHeight: 800,
        titleBarStyle: "overlay",
        decorations: true,
        hiddenTitle: true,
        trafficLightPosition: new LogicalPosition(15, 30),
        visible: false,
        center: true,
      });
      void main
        .once("tauri://error", (event) =>
          failed(new Error(String(event.payload))),
        )
        .then((cleanup) => {
          unlistenError = cleanup;
        })
        .catch(failed);
      await mounted;
    } finally {
      clearTimeout(timeout);
      unlisten();
      unlistenError?.();
    }
  }
  await emitTo("main", "open-expanded-player");
  await main.show();
  await main.unminimize();
  await main.setFocus();
}
