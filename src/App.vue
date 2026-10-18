<script setup lang="ts">
import { computed, onMounted, provide, ref, watch } from "vue";
import AppDialog from "./components/AppDialog.vue";
import StaqSwitcher from "./components/StaqSwitcher.vue";
import TaskCard from "./components/TaskCard.vue";
import WindowChrome from "./components/WindowChrome.vue";
import { useUpdater } from "./useUpdater";
import { useWindowFrame, windowFrameKey } from "./useWindowFrame";
import {
  DEFAULT_STAQ_ID,
  createStaq,
  deleteStaq,
  getStaqs,
  popCurrentTask,
  pushOnStack,
  pushToQueue,
  staqLabel,
  staqTaskCount,
  type StaqSnapshot,
} from "./staq";

type TaskPriority = "regular" | "urgent";

const SELECTED_STAQ_KEY = "staq.selectedStaqId";

const windowFrame = useWindowFrame();
provide(windowFrameKey, windowFrame);
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

function readSelectedStaqId(): number {
  try {
    const value = Number(localStorage.getItem(SELECTED_STAQ_KEY));
    return Number.isInteger(value) ? value : DEFAULT_STAQ_ID;
  } catch {
    return DEFAULT_STAQ_ID;
  }
}

const emptyStaq: StaqSnapshot = {
  id: DEFAULT_STAQ_ID,
  name: null,
  description: null,
  queue: [],
  stack: [],
};

const staqs = ref<StaqSnapshot[]>([]);
const selectedStaqId = ref(readSelectedStaqId());
const staq = computed(
  () => staqs.value.find((item) => item.id === selectedStaqId.value) ?? emptyStaq,
);

const switcherRef = ref<InstanceType<typeof StaqSwitcher> | null>(null);
const taskDialogRef = ref<InstanceType<typeof AppDialog> | null>(null);
const createStaqDialogRef = ref<InstanceType<typeof AppDialog> | null>(null);
const deleteStaqDialogRef = ref<InstanceType<typeof AppDialog> | null>(null);
const updateDialogRef = ref<InstanceType<typeof AppDialog> | null>(null);

const updater = useUpdater();
const {
  update: availableUpdate,
  installing: isInstallingUpdate,
  progress: updateProgress,
  error: updateError,
} = updater;

const dialogMode = ref<TaskPriority>("regular");
const taskTitle = ref("");
const staqName = ref("");
const staqDescription = ref("");
const staqToDeleteId = ref<number | null>(null);
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
const canCreateStaq = computed(
  () => staqName.value.trim().length > 0 && !isBusy.value,
);
const staqToDelete = computed(
  () => staqs.value.find((item) => item.id === staqToDeleteId.value) ?? null,
);
const dialogTitle = computed(() =>
  dialogMode.value === "urgent" ? "Добавить срочную задачу" : "Добавить задачу",
);
const dialogHint = computed(() =>
  dialogMode.value === "urgent"
    ? "Она окажется в самом верху и станет текущей."
    : "Она встанет в конец очереди.",
);

watch(selectedStaqId, (id) => {
  try {
    localStorage.setItem(SELECTED_STAQ_KEY, String(id));
  } catch {
    // The selection is a convenience; the app works without storage.
  }
});

function reportCommandError(message: string, error: unknown) {
  console.error(message, error);
  commandError.value = message;
}

function replaceStaq(updated: StaqSnapshot) {
  staqs.value = staqs.value.map((item) => (item.id === updated.id ? updated : item));
}

function focusSwitcher() {
  switcherRef.value?.focus();
}

function selectStaq(id: number) {
  if (isBusy.value) return;

  commandError.value = null;
  selectedStaqId.value = id;
}

function openDialog(mode: TaskPriority) {
  if (actionsDisabled.value) return;

  commandError.value = null;
  dialogMode.value = mode;
  taskTitle.value = "";
  void taskDialogRef.value?.open();
}

async function addTask() {
  const title = taskTitle.value.trim();
  if (!title || isBusy.value) return;

  isBusy.value = true;
  commandError.value = null;

  try {
    const staqId = selectedStaqId.value;
    replaceStaq(
      dialogMode.value === "urgent"
        ? await pushOnStack(staqId, { name: title })
        : await pushToQueue(staqId, { name: title }),
    );
    taskDialogRef.value?.close();
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
    replaceStaq(await popCurrentTask(selectedStaqId.value));
  } catch (error) {
    reportCommandError("Не удалось завершить задачу. Попробуйте ещё раз.", error);
  } finally {
    isBusy.value = false;
  }
}

function openCreateStaqDialog() {
  if (actionsDisabled.value) return;

  commandError.value = null;
  staqName.value = "";
  staqDescription.value = "";
  void createStaqDialogRef.value?.open(focusSwitcher);
}

async function submitCreateStaq() {
  const name = staqName.value.trim();
  if (!name || isBusy.value) return;

  isBusy.value = true;
  commandError.value = null;

  try {
    const description = staqDescription.value.trim();
    const created = await createStaq({ name, description: description || undefined });
    staqs.value = [...staqs.value, created];
    selectedStaqId.value = created.id;
    createStaqDialogRef.value?.close();
  } catch (error) {
    reportCommandError("Не удалось создать стек. Попробуйте ещё раз.", error);
  } finally {
    isBusy.value = false;
  }
}

function openDeleteStaqDialog(target: StaqSnapshot) {
  if (actionsDisabled.value || target.id === DEFAULT_STAQ_ID) return;

  commandError.value = null;
  staqToDeleteId.value = target.id;
  void deleteStaqDialogRef.value?.open(focusSwitcher);
}

async function confirmDeleteStaq() {
  const target = staqToDelete.value;
  if (!target || isBusy.value) return;

  isBusy.value = true;
  commandError.value = null;

  try {
    await deleteStaq(target.id);
    staqs.value = staqs.value.filter((item) => item.id !== target.id);
    if (selectedStaqId.value === target.id) selectedStaqId.value = DEFAULT_STAQ_ID;
    deleteStaqDialogRef.value?.close();
  } catch (error) {
    reportCommandError("Не удалось удалить стек. Попробуйте ещё раз.", error);
  } finally {
    isBusy.value = false;
  }
}

async function loadStaqs() {
  if (isBusy.value) return;

  isLoading.value = true;
  commandError.value = null;

  try {
    const loaded = await getStaqs();
    staqs.value = loaded;
    if (!loaded.some((item) => item.id === selectedStaqId.value)) {
      selectedStaqId.value = DEFAULT_STAQ_ID;
    }
    hasLoaded.value = true;
  } catch (error) {
    reportCommandError("Не удалось загрузить задачи из приложения.", error);
  } finally {
    isLoading.value = false;
  }
}

async function offerUpdate() {
  if (await updater.checkForUpdate()) void updateDialogRef.value?.open();
}

onMounted(async () => {
  await loadStaqs();
  void offerUpdate();
});
</script>

<template>
  <div class="window-frame" :class="frameClasses">
    <WindowChrome v-if="customFrame" :maximized="maximized" v-on="windowEvents" />
    <p v-if="windowError" class="window-error" role="alert">{{ windowError }}</p>
    <main class="app-shell">
      <header class="app-header">
        <div class="app-title">
          <h1 class="eyebrow">STAQ</h1>

          <div class="queue-summary" aria-live="polite">
            <div class="queue-summary__tasks" v-if="hasLoaded">
              <span>Задач: {{ tasks.length }}</span>

              <span v-if="urgentCount" class="urgent-summary">Срочных: {{ urgentCount }}</span>
            </div>
            <span v-else>Загрузка…</span>
          </div>
        </div>

        <StaqSwitcher
          ref="switcherRef"
          class="app-switcher"
          :staqs="staqs"
          :selected-id="selectedStaqId"
          :disabled="actionsDisabled"
          @select="selectStaq"
          @create="openCreateStaqDialog"
          @delete="openDeleteStaqDialog"
        />
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
            <button class="retry-button" type="button" @click="loadStaqs">Повторить</button>
          </div>

          <!-- Keyed by staq so switching replaces the list instead of animating every task. -->
          <div
            v-else-if="tasks.length"
            :key="selectedStaqId"
            class="task-list"
            aria-label="Список текущих задач"
          >
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

    <AppDialog
      ref="taskDialogRef"
      :busy="isBusy"
      labelledby="dialog-title"
      describedby="dialog-description"
      @close="taskTitle = ''"
    >
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
          v-model="taskTitle"
          data-autofocus
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
            @click="taskDialogRef?.close()"
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
    </AppDialog>

    <AppDialog
      ref="createStaqDialogRef"
      :busy="isBusy"
      labelledby="create-staq-title"
      describedby="create-staq-description"
      @close="staqName = ''; staqDescription = ''"
    >
      <form class="dialog-card" @submit.prevent="submitCreateStaq">
        <div class="dialog-heading">
          <span class="dialog-mode-icon" aria-hidden="true"></span>
          <div>
            <p class="dialog-kicker">Новый стек</p>
            <h2 id="create-staq-title">Создать стек</h2>
          </div>
        </div>

        <p id="create-staq-description" class="dialog-description">
          У стека своя очередь и свои срочные задачи.
        </p>

        <p v-if="commandError" class="dialog-error" role="alert">{{ commandError }}</p>

        <label for="staq-name">Название</label>
        <input
          id="staq-name"
          v-model="staqName"
          data-autofocus
          type="text"
          maxlength="60"
          autocomplete="off"
          placeholder="Например, Работа"
        />

        <label class="dialog-field-label" for="staq-description">Описание (необязательно)</label>
        <input
          id="staq-description"
          v-model="staqDescription"
          type="text"
          maxlength="160"
          autocomplete="off"
        />

        <div class="dialog-actions">
          <button
            class="secondary-button"
            type="button"
            :disabled="isBusy"
            @click="createStaqDialogRef?.close()"
          >
            Отмена
          </button>
          <button class="primary-button" type="submit" :disabled="!canCreateStaq">
            {{ isBusy ? "Создаём…" : "Создать" }}
          </button>
        </div>
      </form>
    </AppDialog>

    <AppDialog
      ref="deleteStaqDialogRef"
      :busy="isBusy"
      labelledby="delete-staq-title"
      describedby="delete-staq-description"
      @close="staqToDeleteId = null"
    >
      <form class="dialog-card" @submit.prevent="confirmDeleteStaq">
        <div class="dialog-heading">
          <span class="dialog-mode-icon dialog-mode-icon--danger" aria-hidden="true"></span>
          <div>
            <p class="dialog-kicker dialog-kicker--danger">Удаление</p>
            <h2 id="delete-staq-title">
              Удалить стек «{{ staqToDelete ? staqLabel(staqToDelete) : "" }}»?
            </h2>
          </div>
        </div>

        <p id="delete-staq-description" class="dialog-description">
          <template v-if="staqToDelete && staqTaskCount(staqToDelete)">
            Вместе с ним будут удалены задачи: {{ staqTaskCount(staqToDelete) }}.
          </template>
          <template v-else>В стеке нет задач.</template>
          Это действие нельзя отменить.
        </p>

        <p v-if="commandError" class="dialog-error" role="alert">{{ commandError }}</p>

        <div class="dialog-actions">
          <button
            class="secondary-button"
            type="button"
            data-autofocus
            :disabled="isBusy"
            @click="deleteStaqDialogRef?.close()"
          >
            Отмена
          </button>
          <button
            class="primary-button primary-button--urgent"
            type="submit"
            :disabled="isBusy || !staqToDelete"
          >
            {{ isBusy ? "Удаляем…" : "Удалить" }}
          </button>
        </div>
      </form>
    </AppDialog>

    <AppDialog
      ref="updateDialogRef"
      :busy="isInstallingUpdate"
      labelledby="update-title"
      describedby="update-description"
      @close="updateError = null"
    >
      <form class="dialog-card" @submit.prevent="updater.install">
        <div class="dialog-heading">
          <span class="dialog-mode-icon" aria-hidden="true"></span>
          <div>
            <p class="dialog-kicker">Обновление</p>
            <h2 id="update-title">Доступна версия {{ availableUpdate?.version }}</h2>
          </div>
        </div>

        <p id="update-description" class="dialog-description">
          Приложение скачает обновление и перезапустится. Задачи сохранятся.
        </p>

        <p v-if="updateError" class="dialog-error" role="alert">{{ updateError }}</p>

        <div class="dialog-actions">
          <button
            class="secondary-button"
            type="button"
            :disabled="isInstallingUpdate"
            @click="updateDialogRef?.close()"
          >
            Позже
          </button>
          <button
            class="primary-button"
            type="submit"
            data-autofocus
            :disabled="isInstallingUpdate"
          >
            <template v-if="isInstallingUpdate">
              Обновляем…{{ updateProgress === null ? "" : ` ${updateProgress}%` }}
            </template>
            <template v-else>Обновить</template>
          </button>
        </div>
      </form>
    </AppDialog>
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
  /* Keep content clear of the status and navigation bars on edge-to-edge mobile screens. */
  padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left);
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
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
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

.app-title {
  flex: 0 0 auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.app-switcher {
  flex: 1 1 200px;
  max-width: 260px;
  margin-top: 6px;
}

.app-header h1 {
  margin: 0;
  font-size: clamp(1.45rem, 4vw, 2rem);
  line-height: 1.08;
  letter-spacing: -0.04em;
}

.queue-summary {
  flex: 0 0 auto;
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

@media (hover: hover) {
  .add-control:hover:not(:disabled) .add-icon {
    transform: scale(1.07);
    background: #d7e8db;
  }

  .add-control--urgent:hover:not(:disabled) .add-icon {
    background: #f7d8cf;
  }
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

@media (hover: hover) {
  .retry-button:hover {
    background: var(--urgent-soft);
  }
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
  padding:
    calc(18px + env(safe-area-inset-top))
    calc(18px + env(safe-area-inset-right))
    calc(18px + env(safe-area-inset-bottom))
    calc(18px + env(safe-area-inset-left));
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

.dialog-mode-icon--danger {
  color: var(--urgent);
  background: var(--urgent-soft);
}

.dialog-mode-icon--danger::before,
.dialog-mode-icon--danger::after {
  transform: translate(-50%, -50%) rotate(45deg);
}

.dialog-kicker--danger {
  color: var(--urgent-dark);
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

.dialog-card .dialog-field-label {
  margin-top: 14px;
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

@media (hover: hover) {
  .primary-button:hover:not(:disabled) {
    background: var(--green-dark);
  }
}

.primary-button--urgent {
  border-color: var(--urgent);
  background: var(--urgent);
}

@media (hover: hover) {
  .primary-button--urgent:hover:not(:disabled) {
    background: var(--urgent-dark);
  }
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
  .queue-summary__tasks {
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
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
