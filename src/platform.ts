import { isTauri, type PluginListener } from "@tauri-apps/api/core";
import { onBackButtonPress } from "@tauri-apps/api/app";

export const isMobile = isTauri() && /Android|iPhone|iPad/i.test(navigator.userAgent);

// Most recently opened layer (dialog, menu) handles Back first.
const backHandlers: Array<() => void> = [];
let backListener: Promise<PluginListener | null> | null = null;

function syncBackListener() {
  if (backHandlers.length && !backListener) {
    backListener = onBackButtonPress(() => backHandlers[backHandlers.length - 1]?.()).catch((cause) => {
      console.error("Back button listener failed", cause);
      return null;
    });
  } else if (!backHandlers.length && backListener) {
    // While a listener exists Android skips its default Back action, so drop it
    // as soon as nothing is open to let Back leave the app again.
    const listener = backListener;
    backListener = null;
    void listener.then((registered) => registered?.unregister());
  }
}

export function pushBackHandler(handler: () => void): () => void {
  if (!isMobile) return () => {};
  backHandlers.push(handler);
  syncBackListener();
  return () => {
    const index = backHandlers.lastIndexOf(handler);
    if (index === -1) return;
    backHandlers.splice(index, 1);
    syncBackListener();
  };
}
