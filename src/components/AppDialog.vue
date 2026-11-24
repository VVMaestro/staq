<script setup lang="ts">
import { computed, inject, nextTick, ref } from "vue";
import WindowChrome from "./WindowChrome.vue";
import { windowFrameKey } from "../useWindowFrame";
import { pushBackHandler } from "../platform";

const props = defineProps<{
  busy: boolean;
  labelledby: string;
  describedby?: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

const windowFrame = inject(windowFrameKey);
if (!windowFrame) throw new Error("AppDialog must be used inside a window frame provider");
const { enabled: customFrame, maximized, error: windowError } = windowFrame;
const windowEvents = {
  minimize: windowFrame.minimize,
  toggleMaximize: windowFrame.toggleMaximize,
  close: windowFrame.close,
  resize: windowFrame.resize,
};
const frameClasses = computed(() => ({
  'window-frame--custom': customFrame.value,
  'window-frame--maximized': maximized.value,
}));

const dialogRef = ref<HTMLDialogElement | null>(null);
let restoreFocus: (() => void) | null = null;
let releaseBack: (() => void) | null = null;

async function open(onClosed?: () => void) {
  const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  restoreFocus = onClosed ?? (() => {
    if (trigger?.isConnected) trigger.focus();
  });
  dialogRef.value?.showModal();
  releaseBack ??= pushBackHandler(() => {
    if (!props.busy) close();
  });
  await nextTick();
  dialogRef.value?.querySelector<HTMLElement>("[data-autofocus]")?.focus();
}

function close() {
  dialogRef.value?.close();
}

function handleClose() {
  releaseBack?.();
  releaseBack = null;
  emit("close");
  const focus = restoreFocus;
  restoreFocus = null;
  if (focus) void nextTick(focus);
}

function handleBackdrop(event: MouseEvent) {
  if (!props.busy && event.target === event.currentTarget) close();
}

function handleCancel(event: Event) {
  if (props.busy) event.preventDefault();
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key !== "Tab") return;
  const controls = dialogRef.value?.querySelectorAll<HTMLElement>(
    "button:not(:disabled), input:not(:disabled), textarea:not(:disabled)",
  );
  if (!controls?.length) return;
  const first = controls[0];
  const last = controls[controls.length - 1];
  // Keep Tab inside the modal instead of handing focus to browser chrome.
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

defineExpose({ open, close });
</script>

<template>
  <dialog
    ref="dialogRef"
    class="task-dialog"
    :class="frameClasses"
    :aria-labelledby="labelledby"
    :aria-describedby="describedby"
    @cancel="handleCancel"
    @keydown="handleKeydown"
    @close="handleClose"
  >
    <WindowChrome v-if="customFrame" :maximized="maximized" v-on="windowEvents" />
    <p v-if="windowError" class="window-error" role="alert">{{ windowError }}</p>
    <div class="dialog-workspace" @click="handleBackdrop">
      <slot />
    </div>
  </dialog>
</template>
