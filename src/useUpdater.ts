import { ref, shallowRef } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { isMobile } from "./platform";

export function useUpdater() {
  // Update wraps a native resource, so keep it out of Vue's deep proxies.
  const update = shallowRef<Update | null>(null);
  const installing = ref(false);
  const progress = ref<number | null>(null);
  const error = ref<string | null>(null);

  async function checkForUpdate(): Promise<Update | null> {
    // The updater plugin is registered on desktop only; mobile updates come from the store.
    if (!isTauri() || isMobile) return null;

    try {
      update.value = await check();
    } catch (cause) {
      // A failed check must never block the app, e.g. offline or before the first release.
      console.error("Update check failed", cause);
      update.value = null;
    }
    return update.value;
  }

  async function install() {
    const pending = update.value;
    if (!pending || installing.value) return;

    installing.value = true;
    progress.value = null;
    error.value = null;

    try {
      let total = 0;
      let downloaded = 0;
      await pending.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total) progress.value = Math.min(100, Math.round((downloaded / total) * 100));
        }
      });
      // Windows exits inside the installer; other platforms need an explicit restart.
      await relaunch();
    } catch (cause) {
      console.error("Update install failed", cause);
      error.value = "Не удалось установить обновление. Попробуйте позже.";
      installing.value = false;
    }
  }

  return { update, installing, progress, error, checkForUpdate, install };
}
