import { onMounted, onUnmounted, ref } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow, type Window } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";

export type ResizeDirection = Parameters<Window["startResizeDragging"]>[0];

export function useWindowFrame() {
  const enabled = ref(false);
  const maximized = ref(false);
  const error = ref<string | null>(null);
  const appWindow = isTauri() ? getCurrentWindow() : null;
  let disposed = false;
  let unlisten: UnlistenFn | undefined;
  let revision = 0;

  async function syncMaximized() {
    if (!appWindow) return;
    const currentRevision = ++revision;
    const value = await appWindow.isMaximized();
    if (!disposed && currentRevision === revision) maximized.value = value;
  }

  async function run(action: () => Promise<unknown>) {
    error.value = null;
    try {
      await action();
    } catch (cause) {
      console.error("Window operation failed", cause);
      if (!disposed) error.value = "Не удалось выполнить действие с окном. Попробуйте ещё раз.";
    }
  }

  onMounted(() => {
    void run(async () => {
      if (!appWindow) return;
      const decorated = await appWindow.isDecorated();
      if (disposed || decorated) return;
      enabled.value = true;
      const stop = await appWindow.onResized(() => { void run(syncMaximized); });
      if (disposed) {
        stop();
        return;
      }
      unlisten = stop;
      await syncMaximized();
    });
  });

  onUnmounted(() => {
    disposed = true;
    unlisten?.();
  });

  return {
    enabled,
    maximized,
    error,
    minimize: () => run(async () => { await appWindow?.minimize(); }),
    toggleMaximize: () => run(async () => {
      await appWindow?.toggleMaximize();
      await syncMaximized();
    }),
    close: () => run(async () => { await appWindow?.close(); }),
    resize: (direction: ResizeDirection) => run(async () => {
      if (!maximized.value) await appWindow?.startResizeDragging(direction);
    }),
  };
}
