<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from "vue";
import TaskCard from "./components/TaskCard.vue";
import WindowChrome from "./components/WindowChrome.vue";
import { useWindowFrame } from "./useWindowFrame";
import {
  getStaq,
  popCurrentTask,
  pushOnStack,
  pushToQueue,
  type StaqSnapshot,
} from "./staq";

type TaskPriority = "regular" | "urgent";

const windowFrame = useWindowFrame();
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

const staq = ref<StaqSnapshot>({ queue: [], stack: [] });
const dialogRef = ref<HTMLDialogElement | null>(null);
const inputRef = ref<HTMLInputElement | null>(null);
let dialogTrigger: HTMLElement | null = null;
const dialogMode = ref<TaskPriority>("regular");
const taskTitle = ref("");
const isLoading = ref(true);
const isBusy = ref(false);
const hasLoaded = ref(false);
const commandError = ref<string | null>(null);

const urgentTasks = computed(() => [...staq.value.stack].reverse());
const regularTasks = computed(() => staq.value.queue);
const tasks = computed(() => [...urgentTasks.value, ...regularTasks.value]);
const taskGroups = computed(() => [
  {
    priority: "urgent" as const,
    tasks: urgentTasks.value,
    framed: false,
  },
  {
    priority: "regular" as const,
    tasks: regularTasks.value,
    framed: urgentTasks.value.length > 0 && regularTasks.value.length > 0,
  },
]);
const urgentCount = computed(() => urgentTasks.value.length);
const currentTaskId = computed(() => tasks.value[0]?.id ?? null);
const actionsDisabled = computed(
  () => isLoading.value || isBusy.value || !hasLoaded.value,
);
const canSubmit = computed(
  () => taskTitle.value.trim().length > 0 && !isBusy.value,
);
const dialogTitle = computed(() =>
  dialogMode.value === "urgent" ? "Добавить срочную задачу" : "Добавить задачу",
);
const dialogHint = computed(() =>
  dialogMode.value === "urgent"
    ? "Она окажется в самом верху и станет текущей."
    : "Она встанет в конец очереди.",
);

function reportCommandError(message: string, error: unknown) {
  console.error(message, error);
  commandError.value = message;
}

async function openDialog(mode: TaskPriority) {
  if (actionsDisabled.value) return;

  commandError.value = null;
  dialogMode.value = mode;
  taskTitle.value = "";
  dialogTrigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  dialogRef.value?.showModal();
  await nextTick();
  inputRef.value?.focus();
}

function closeDialog() {
  dialogRef.value?.close();
}

function resetForm() {
  taskTitle.value = "";
  void nextTick(() => {
    if (dialogTrigger?.isConnected) dialogTrigger.focus();
  });
}

async function addTask() {
  const title = taskTitle.value.trim();
  if (!title || isBusy.value) return;

  isBusy.value = true;
  commandError.value = null;

  try {
    staq.value =
      dialogMode.value === "urgent"
        ? await pushOnStack(title)
        : await pushToQueue(title);
    closeDialog();
  } catch (error) {
    reportCommandError("Не удалось добавить задачу. Попробуйте ещё раз.", error);
  } finally {
    isBusy.value = false;
  }
}

async function completeCurrentTask() {
  if (isBusy.value || currentTaskId.value === null) return;

  isBusy.value = true;
  commandError.value = null;

  try {
    staq.value = await popCurrentTask();
  } catch (error) {
    reportCommandError("Не удалось завершить задачу. Попробуйте ещё раз.", error);
  } finally {
    isBusy.value = false;
  }
}

function handleDialogBackdrop(event: MouseEvent) {
  if (!isBusy.value && event.target === event.currentTarget) closeDialog();
}

function handleDialogCancel(event: Event) {
  if (isBusy.value) event.preventDefault();
}

function handleDialogKeydown(event: KeyboardEvent) {
  if (event.key !== "Tab") return;
  const controls = dialogRef.value?.querySelectorAll<HTMLElement>(
    "button:not(:disabled), input:not(:disabled)",
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

async function loadStaq() {
  if (isBusy.value) return;

  isLoading.value = true;
  commandError.value = null;

  try {
    staq.value = await getStaq();
    hasLoaded.value = true;
  } catch (error) {
    reportCommandError("Не удалось загрузить задачи из приложения.", error);
  } finally {
    isLoading.value = false;
  }
}

onMounted(loadStaq);
</script>

<template>
  <div class="window-frame" :class="frameClasses">
    <WindowChrome v-if="customFrame" :maximized="maximized" v-on="windowEvents" />
    <p v-if="windowError" class="window-error" role="alert">{{ windowError }}</p>
    <main class="app-shell">
      <header class="app-header">
        <div>
          <h1 class="eyebrow">STAQ</h1>
        </div>

        <div class="queue-summary" aria-live="polite">
          <div class="queue-summary__tasks" v-if="hasLoaded">
            <span>Задач: {{ tasks.length }}</span>

            <span v-if="urgentCount" class="urgent-summary">Срочных: {{ urgentCount }}</span>
          </div>
          <span v-else>Загрузка…</span>
        </div>
      </header>

      <section class="queue-workspace" aria-labelledby="queue-title">
        <button
          class="add-control add-control--urgent"
          type="button"
          aria-label="Добавить срочную задачу в начало очереди"
          :disabled="actionsDisabled"
          @click="openDialog('urgent')"
        >
          <span class="add-icon" aria-hidden="true"></span>
          <span>
            <strong>Срочная задача</strong>
            <small>добавить наверх</small>
          </span>
        </button>

        <p v-if="commandError && hasLoaded" class="command-error" role="alert">
          {{ commandError }}
        </p>

        <div class="queue-container" :aria-busy="isLoading || isBusy">
          <div v-if="isLoading" class="empty-state" aria-live="polite">
            <span class="empty-mark" aria-hidden="true">…</span>
            <h3>Загружаем задачи</h3>
          </div>

          <div v-else-if="!hasLoaded" class="empty-state empty-state--error" role="alert">
            <span class="empty-mark" aria-hidden="true">!</span>
            <h3>Задачи недоступны</h3>
            <p>{{ commandError }}</p>
            <button class="retry-button" type="button" @click="loadStaq">Повторить</button>
          </div>

          <div v-else-if="tasks.length" class="task-list" aria-label="Список текущих задач">
            <TransitionGroup
              v-for="group in taskGroups"
              :key="group.priority"
              name="task"
              tag="div"
              class="task-group"
              :class="[
                `task-group--${group.priority}`,
                { 'task-group--framed': group.framed },
              ]"
            >
              <TaskCard
                v-for="task in group.tasks"
                :key="task.id"
                :title="task.name"
                :urgent="group.priority === 'urgent'"
                :current="task.id === currentTaskId"
                :busy="isBusy"
                @complete="completeCurrentTask"
              />
            </TransitionGroup>
          </div>

          <div v-else class="empty-state">
            <span class="empty-mark" aria-hidden="true">0</span>
            <h3>Очередь свободна</h3>
            <p>Добавьте первую задачу снизу или срочную — сверху.</p>
          </div>
        </div>

        <button
          class="add-control add-control--regular"
          type="button"
          aria-label="Добавить обычную задачу в конец очереди"
          :disabled="actionsDisabled"
          @click="openDialog('regular')"
        >
          <span class="add-icon" aria-hidden="true"></span>
          <span>
            <strong>Обычная задача</strong>
            <small>добавить в конец</small>
          </span>
        </button>
      </section>

    </main>

    <dialog
      ref="dialogRef"
      class="task-dialog"
      :class="frameClasses"
      aria-labelledby="dialog-title"
      aria-describedby="dialog-description"
      @cancel="handleDialogCancel"
      @keydown="handleDialogKeydown"
      @close="resetForm"
    >
      <WindowChrome v-if="customFrame" :maximized="maximized" v-on="windowEvents" />
      <p v-if="windowError" class="window-error" role="alert">{{ windowError }}</p>
      <div class="dialog-workspace" @click="handleDialogBackdrop">
        <form class="dialog-card" @submit.prevent="addTask">
          <div class="dialog-heading">
            <span
              class="dialog-mode-icon"
              :class="{ 'dialog-mode-icon--urgent': dialogMode === 'urgent' }"
              aria-hidden="true"
            ></span>
            <div>
              <p class="dialog-kicker">
                {{ dialogMode === "urgent" ? "Наверх стека" : "В конец очереди" }}
              </p>
              <h2 id="dialog-title">{{ dialogTitle }}</h2>
            </div>
          </div>

          <p id="dialog-description" class="dialog-description">{{ dialogHint }}</p>

          <p v-if="commandError" class="dialog-error" role="alert">{{ commandError }}</p>

          <label for="task-title">Коротко опишите задачу</label>
          <input
            id="task-title"
            ref="inputRef"
            v-model="taskTitle"
            type="text"
            maxlength="160"
            autocomplete="off"
            placeholder="Например, ответить на письмо"
          />

          <div class="dialog-actions">
            <button
              class="secondary-button"
              type="button"
              :disabled="isBusy"
              @click="closeDialog"
            >
              Отмена
            </button>
            <button
              class="primary-button"
              :class="{ 'primary-button--urgent': dialogMode === 'urgent' }"
              type="submit"
              :disabled="!canSubmit"
            >
              {{ isBusy ? "Добавляем…" : "Добавить" }}
            </button>
          </div>
        </form>
      </div>
    </dialog>
  </div>
</template>

<style>
:root {
  font-family:
    Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  color: #20251f;
  background: transparent;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  --ink: #20251f;
  --muted: #71786e;
  --surface: #fbfcf9;
  --line: #dce1d8;
  --green: #2f6b4e;
  --green-dark: #24533d;
  --green-soft: #e3eee6;
  --urgent: #c54b32;
  --urgent-dark: #a83b27;
  --urgent-soft: #fbe8e2;
}

* {
  box-sizing: border-box;
}

html,
body,
#app {
  width: 100%;
  height: 100%;
  background: transparent;
}

body {
  margin: 0;
  min-width: 320px;
  overflow: hidden;
}

button,
input {
  font: inherit;
}

button {
  color: inherit;
}

button:focus-visible,
input:focus-visible {
  outline: 3px solid rgba(47, 107, 78, 0.28);
  outline-offset: 2px;
}

.window-frame {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background:
    radial-gradient(circle at 7% 5%, rgba(255, 255, 255, 0.95), transparent 30%),
    linear-gradient(145deg, #f5f6f2 0%, #e9ede6 100%);
}

.window-frame--custom {
  border-radius: 10px;
  clip-path: inset(0 round 10px);
}

.window-frame--maximized {
  border-radius: 0;
  clip-path: none;
}

.window-error {
  flex-shrink: 0;
  margin: 0;
  padding: 6px 14px;
  color: var(--urgent-dark);
  background: var(--urgent-soft);
  font-size: 0.75rem;
}

.app-shell {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 24px clamp(20px, 6vw, 54px);
}

.app-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  max-width: 680px;
  width: 100%;
  margin: 0 auto;
}

.eyebrow,
.section-kicker,
.dialog-kicker {
  margin: 0 0 3px;
  color: var(--green);
  font-size: 0.67rem;
  font-weight: 800;
  letter-spacing: 0.16em;
  text-transform: uppercase;
}

.app-header h1 {
  margin: 0;
  font-size: clamp(1.45rem, 4vw, 2rem);
  line-height: 1.08;
  letter-spacing: -0.04em;
}

.queue-summary {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--muted);
  font-size: 0.78rem;
  font-weight: 700;
}

.queue-summary__tasks {
  display: flex;
  align-items: center;
  gap: 8px;
}

.urgent-summary {
  padding: 4px 8px;
  border-radius: 999px;
  color: var(--urgent-dark);
  background: var(--urgent-soft);
}

.queue-workspace {
  flex: 1;
  min-height: 0;
  width: min(100%, 680px);
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 18px;
  border: 1px solid rgba(255, 255, 255, 0.9);
  border-radius: 24px;
  background: rgba(251, 252, 249, 0.78);
  box-shadow:
    0 20px 55px rgba(46, 57, 43, 0.1),
    inset 0 0 0 1px rgba(217, 223, 213, 0.6);
  backdrop-filter: blur(12px);
}

.section-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.section-heading h2 {
  margin: 0;
  font-size: 1.1rem;
  letter-spacing: -0.02em;
}

.section-kicker {
  color: var(--muted);
  letter-spacing: 0.08em;
}

.flow-label {
  padding: 5px 8px;
  border: 1px solid var(--line);
  border-radius: 7px;
  color: var(--muted);
  background: rgba(255, 255, 255, 0.55);
  font-size: 0.65rem;
  font-weight: 800;
  letter-spacing: 0.08em;
}

.add-control {
  width: fit-content;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0;
  border: 0;
  background: transparent;
  cursor: pointer;
  text-align: left;
}

.add-control--regular {
  align-self: flex-end;
}

.add-icon {
  position: relative;
  width: 31px;
  height: 31px;
  display: grid;
  place-items: center;
  border: 1px solid #bed0c2;
  border-radius: 50%;
  color: var(--green);
  background: var(--green-soft);
  transition: transform 160ms ease, background-color 160ms ease;
}

.add-icon::before,
.add-icon::after,
.dialog-mode-icon::before,
.dialog-mode-icon::after {
  content: "";
  position: absolute;
  top: 50%;
  left: 50%;
  border-radius: 999px;
  background: currentColor;
  transform: translate(-50%, -50%);
}

.add-icon::before {
  width: 12px;
  height: 2px;
}

.add-icon::after {
  width: 2px;
  height: 12px;
}

.add-control--urgent .add-icon {
  color: var(--urgent);
  border-color: #ebc0b5;
  background: var(--urgent-soft);
}

.add-control:hover:not(:disabled) .add-icon {
  transform: scale(1.07);
  background: #d7e8db;
}

.add-control--urgent:hover:not(:disabled) .add-icon {
  background: #f7d8cf;
}

.add-control:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.add-control strong,
.add-control small {
  display: block;
}

.add-control strong {
  font-size: 0.76rem;
}

.add-control small {
  margin-top: 1px;
  color: var(--muted);
  font-size: 0.66rem;
}

.queue-container {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  border: 1px dashed #cbd2c7;
  border-radius: 15px;
  background: rgba(240, 243, 237, 0.7);
}

.command-error,
.dialog-error {
  margin: 0;
  color: var(--urgent-dark);
  font-size: 0.73rem;
  line-height: 1.4;
}

.command-error {
  padding: 7px 9px;
  border: 1px solid #efc7bd;
  border-radius: 9px;
  background: var(--urgent-soft);
}

.task-list {
  height: 100%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
  scrollbar-color: #bdc7b9 transparent;
  scrollbar-width: thin;
}

.task-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.task-group:empty {
  display: none;
}

.task-group--framed {
  padding: 8px;
  border: 1px solid #c5cec2;
  border-radius: 14px;
}

.empty-state {
  height: 100%;
  min-height: 140px;
  display: grid;
  place-content: center;
  justify-items: center;
  padding: 24px;
  color: var(--muted);
  text-align: center;
}

.empty-mark {
  width: 42px;
  height: 42px;
  display: grid;
  place-items: center;
  margin-bottom: 9px;
  border: 1px dashed #b7c0b3;
  border-radius: 13px;
  color: #9aa497;
  font-size: 0.76rem;
  font-weight: 800;
}

.empty-state h3 {
  margin: 0;
  color: #525a50;
  font-size: 0.95rem;
}

.empty-state p {
  max-width: 270px;
  margin: 5px 0 0;
  font-size: 0.73rem;
  line-height: 1.45;
}

.empty-state--error .empty-mark {
  color: var(--urgent-dark);
  border-color: #dfafa4;
  background: var(--urgent-soft);
}

.retry-button {
  margin-top: 12px;
  padding: 7px 11px;
  border: 1px solid #d5aaa0;
  border-radius: 8px;
  color: var(--urgent-dark);
  background: #fff8f6;
  font-size: 0.72rem;
  font-weight: 800;
  cursor: pointer;
}

.retry-button:hover {
  background: var(--urgent-soft);
}

.task-enter-from.task-card--urgent,
.task-leave-to.task-card--urgent {
  transform: translateY(-12px);
  opacity: 0;
}

.task-enter-from.task-card--regular,
.task-leave-to.task-card--regular {
  transform: translateY(12px);
  opacity: 0;
}

.task-move {
  transition: transform 220ms ease;
}

.task-dialog {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  max-width: none;
  max-height: none;
  margin: 0;
  padding: 0;
  border: 0;
  overflow: hidden;
  color: var(--ink);
  background: transparent;
}

.task-dialog::backdrop {
  background: transparent;
}

.task-dialog[open] {
  display: flex;
  flex-direction: column;
}

.dialog-workspace {
  flex: 1;
  min-height: 0;
  display: flex;
  padding: 18px;
  overflow: auto;
  background: rgba(31, 38, 30, 0.42);
  backdrop-filter: blur(4px);
}

.dialog-card {
  flex-shrink: 0;
  width: min(440px, 100%);
  margin: auto;
  padding: 24px;
  border: 1px solid rgba(255, 255, 255, 0.8);
  border-radius: 20px;
  background: #fbfcf9;
  box-shadow: 0 28px 90px rgba(30, 37, 29, 0.25);
}

.task-dialog[open] .dialog-card {
  animation: dialog-in 180ms ease-out;
}

.dialog-heading {
  display: flex;
  align-items: center;
  gap: 12px;
}

.dialog-heading h2 {
  margin: 0;
  font-size: 1.2rem;
  letter-spacing: -0.025em;
}

.dialog-mode-icon {
  position: relative;
  flex: 0 0 auto;
  width: 38px;
  height: 38px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  color: var(--green);
  background: var(--green-soft);
}

.dialog-mode-icon::before {
  width: 13px;
  height: 2px;
}

.dialog-mode-icon::after {
  width: 2px;
  height: 13px;
}

.dialog-mode-icon--urgent {
  color: var(--urgent);
  background: var(--urgent-soft);
}

.dialog-description {
  margin: 16px 0 18px;
  color: var(--muted);
  font-size: 0.8rem;
}

.dialog-error {
  margin: -8px 0 16px;
}

.dialog-card label {
  display: block;
  margin-bottom: 7px;
  color: #4d554b;
  font-size: 0.72rem;
  font-weight: 750;
}

.dialog-card input {
  width: 100%;
  padding: 11px 12px;
  border: 1px solid #cfd6cc;
  border-radius: 10px;
  color: var(--ink);
  background: #fff;
  font-size: 0.86rem;
  transition: border-color 150ms ease, box-shadow 150ms ease;
}

.dialog-card input::placeholder {
  color: #9ca39a;
}

.dialog-card input:focus {
  border-color: #7ca089;
  box-shadow: 0 0 0 3px rgba(47, 107, 78, 0.1);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
}

.secondary-button,
.primary-button {
  padding: 9px 14px;
  border-radius: 9px;
  font-size: 0.76rem;
  font-weight: 800;
  cursor: pointer;
}

.secondary-button {
  border: 1px solid var(--line);
  background: #f3f5f1;
}

.secondary-button:disabled {
  cursor: wait;
  opacity: 0.5;
}

.primary-button {
  border: 1px solid var(--green);
  color: #fff;
  background: var(--green);
}

.primary-button:hover:not(:disabled) {
  background: var(--green-dark);
}

.primary-button--urgent {
  border-color: var(--urgent);
  background: var(--urgent);
}

.primary-button--urgent:hover:not(:disabled) {
  background: var(--urgent-dark);
}

.primary-button:disabled {
  cursor: not-allowed;
  opacity: 0.42;
}

@keyframes dialog-in {
  from {
    transform: translateY(8px) scale(0.98);
    opacity: 0;
  }
}

@media (max-height: 620px) {
  .app-shell {
    gap: 12px;
    padding-top: 16px;
    padding-bottom: 16px;
  }

  .queue-workspace {
    padding: 14px;
  }
}

@media (max-width: 520px) {
  .app-header {
    align-items: flex-start;
  }

  .queue-summary {
    flex-direction: column;
    align-items: flex-end;
  }

  .queue-workspace {
    padding: 14px;
    border-radius: 19px;
  }

}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    scroll-behavior: auto !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
</style>
