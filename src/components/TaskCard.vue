<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { pushBackHandler } from "../platform";
import { parseDescriptionSegments } from "../taskDescription";
import { useTaskSwipe } from "../useTaskSwipe";

const props = defineProps<{
  name: string;
  description: string;
  urgent: boolean;
  current: boolean;
  busy: boolean;
  menuOpen: boolean;
}>();

const emit = defineEmits<{
  complete: [];
  edit: [];
  cancel: [];
  openLink: [href: string];
  openMenu: [];
  closeMenu: [];
}>();

const expanded = ref(false);
const surfaceRef = ref<HTMLElement | null>(null);
const menuButtonRef = ref<HTMLButtonElement | null>(null);
const menuRef = ref<HTMLElement | null>(null);
const menuStyle = ref<Record<string, string>>({});

let releaseBack: (() => void) | null = null;
let pendingMenuFocus: "first" | "last" = "first";

const hasDescription = computed(() => props.description.length > 0);
const isExpanded = computed(() => hasDescription.value && expanded.value);
const segments = computed(() => parseDescriptionSegments(props.description));

const {
  isDragging,
  swipeAction,
  swipeReady,
  surfaceStyle,
  consumeSuppressedClick,
  handlePointerDown,
  handlePointerMove,
  handlePointerUp,
  handlePointerCancel,
} = useTaskSwipe({
  surfaceRef,
  current: () => props.current,
  busy: () => props.busy,
  menuOpen: () => props.menuOpen,
  closeMenu: () => closeMenu(false),
  onOutcome: (action) => {
    if (action === "cancel") emit("cancel");
    else emit("complete");
  },
});

watch(hasDescription, (value) => {
  if (!value) expanded.value = false;
});

watch(
  () => props.busy,
  (busy) => {
    if (busy && props.menuOpen) {
      emit("closeMenu");
    }
  },
);

function menuItems(): HTMLElement[] {
  return Array.from(menuRef.value?.querySelectorAll<HTMLElement>("[data-menu-item]") ?? []);
}

function updateMenuPosition() {
  const trigger = menuButtonRef.value;
  if (!trigger) return;

  const rect = trigger.getBoundingClientRect();
  const margin = 12;
  const gap = 6;
  const estimatedHeight = 96;
  const right = Math.max(margin, window.innerWidth - rect.right);
  const openAbove = rect.bottom + gap + estimatedHeight > window.innerHeight - margin;

  menuStyle.value = openAbove
    ? {
        right: `${right}px`,
        bottom: `${Math.max(margin, window.innerHeight - rect.top + gap)}px`,
      }
    : {
        right: `${right}px`,
        top: `${rect.bottom + gap}px`,
      };
}

function detachMenuListeners() {
  document.removeEventListener("pointerdown", handleOutsidePointer, true);
  document.removeEventListener("keydown", handleDocumentKeydown);
  window.removeEventListener("resize", handleViewportChange);
  window.removeEventListener("scroll", handleViewportChange, true);
  releaseBack?.();
  releaseBack = null;
}

function closeMenu(restoreFocus = false) {
  if (!props.menuOpen) return;
  emit("closeMenu");
  if (restoreFocus) {
    void nextTick(() => menuButtonRef.value?.focus());
  }
}

function handleOutsidePointer(event: PointerEvent) {
  const target = event.target as Node | null;
  if (menuRef.value?.contains(target) || menuButtonRef.value?.contains(target)) return;
  closeMenu(false);
}

function handleDocumentKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") return;
  event.preventDefault();
  closeMenu(true);
}

function handleViewportChange() {
  closeMenu(false);
}

watch(
  () => props.menuOpen,
  async (open) => {
    if (open) {
      updateMenuPosition();
      document.addEventListener("pointerdown", handleOutsidePointer, true);
      document.addEventListener("keydown", handleDocumentKeydown);
      window.addEventListener("resize", handleViewportChange);
      window.addEventListener("scroll", handleViewportChange, true);
      releaseBack = pushBackHandler(() => closeMenu(true));

      await nextTick();
      const items = menuItems();
      const focusTarget =
        pendingMenuFocus === "last" ? items[items.length - 1] : items[0];
      pendingMenuFocus = "first";
      focusTarget?.focus();
    } else {
      detachMenuListeners();
    }
  },
);

function toggleMenu() {
  if (props.busy) return;
  if (props.menuOpen) {
    closeMenu(true);
  } else {
    pendingMenuFocus = "first";
    emit("openMenu");
  }
}

function openMenuFromKey(focus: "first" | "last") {
  if (props.busy) return;
  pendingMenuFocus = focus;
  if (!props.menuOpen) {
    emit("openMenu");
  }
}

function handleMenuKeydown(event: KeyboardEvent) {
  const items = menuItems();
  if (!items.length) return;
  const index = items.indexOf(document.activeElement as HTMLElement);

  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      items[(index + 1) % items.length]?.focus();
      break;
    case "ArrowUp":
      event.preventDefault();
      items[index <= 0 ? items.length - 1 : index - 1]?.focus();
      break;
    case "Home":
      event.preventDefault();
      items[0]?.focus();
      break;
    case "End":
      event.preventDefault();
      items[items.length - 1]?.focus();
      break;
    case "Tab":
      closeMenu(false);
      break;
  }
}

function handleEditAction() {
  emit("closeMenu");
  emit("edit");
}

function handleCancelAction() {
  emit("closeMenu");
  emit("cancel");
}

function toggleExpanded() {
  if (!hasDescription.value) return;
  expanded.value = !expanded.value;
}

function handleCardClick(event: MouseEvent) {
  if (consumeSuppressedClick()) return;
  if (!hasDescription.value) return;
  if (event.target instanceof Element && event.target.closest("a, button")) return;
  const selection = window.getSelection();
  if (selection && !selection.isCollapsed) return;
  toggleExpanded();
}

onBeforeUnmount(() => {
  detachMenuListeners();
});
</script>

<template>
  <div
    class="task-swipe"
    :class="{
      'task-card--urgent': urgent,
      'task-card--regular': !urgent,
      'task-swipe--active': swipeAction !== null,
    }"
  >
    <div
      v-if="swipeAction"
      class="task-swipe-backdrop"
      :class="[
        `task-swipe-backdrop--${swipeAction}`,
        { 'task-swipe-backdrop--ready': swipeReady },
      ]"
      aria-hidden="true"
    >
      <span class="swipe-cue">
        <span class="swipe-cue-icon">{{ swipeAction === "complete" ? "✓" : "✕" }}</span>
        <span>{{ swipeAction === "complete" ? "Готово" : "Отмена" }}</span>
      </span>
    </div>

    <article
      ref="surfaceRef"
      class="task-surface"
      :class="{
        'task-surface--current': current,
        'task-surface--urgent': urgent,
        'task-surface--expandable': hasDescription,
        'task-surface--expanded': isExpanded,
        'task-surface--dragging': isDragging,
      }"
      :style="surfaceStyle"
      @click="handleCardClick"
      @pointerdown="handlePointerDown"
      @pointermove="handlePointerMove"
      @pointerup="handlePointerUp"
      @pointercancel="handlePointerCancel"
    >
      <div class="task-copy">
        <div class="task-labels">
          <span v-if="current" class="status-badge">Сейчас</span>
          <span v-if="urgent" class="priority-badge">Срочно</span>
        </div>
        <p class="task-name">{{ name }}</p>
        <p
          v-if="hasDescription"
          class="task-description"
          :class="{ 'task-description--expanded': isExpanded }"
        >
          <template v-for="(segment, index) in segments" :key="index">
            <a
              v-if="segment.type === 'link'"
              class="task-link"
              :href="segment.href"
              target="_blank"
              rel="noopener noreferrer"
              @click.prevent.stop="emit('openLink', segment.href)"
              @keydown.stop
            >{{ segment.text }}</a>
            <template v-else>{{ segment.text }}</template>
          </template>
        </p>
      </div>

      <div class="task-actions">
        <button
          ref="menuButtonRef"
          class="menu-button"
          :class="{ 'menu-button--open': menuOpen }"
          type="button"
          aria-haspopup="menu"
          :aria-expanded="menuOpen"
          :aria-label="`Действия для задачи: ${name}`"
          title="Действия"
          :disabled="busy"
          @click.stop="toggleMenu"
          @keydown.down.prevent="openMenuFromKey('first')"
          @keydown.up.prevent="openMenuFromKey('last')"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
            <circle cx="8" cy="3.5" r="1.25" fill="currentColor" />
            <circle cx="8" cy="8" r="1.25" fill="currentColor" />
            <circle cx="8" cy="12.5" r="1.25" fill="currentColor" />
          </svg>
        </button>

        <button
          v-if="current"
          class="complete-button"
          type="button"
          :aria-label="`Завершить задачу: ${name}`"
          :disabled="busy"
          @click.stop="emit('complete')"
        >
          <span aria-hidden="true">✓</span>
          Готово
        </button>
      </div>
    </article>

    <Teleport to="body">
      <div
        v-if="menuOpen"
        ref="menuRef"
        class="task-menu"
        :style="menuStyle"
        role="menu"
        :aria-label="`Действия для задачи: ${name}`"
        @keydown="handleMenuKeydown"
      >
        <button
          class="task-menu-item"
          type="button"
          role="menuitem"
          data-menu-item
          @click="handleEditAction"
        >
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" focusable="false">
            <path
              d="M11.6 2.4a1.5 1.5 0 0 1 2.1 2.1L6.1 12.1l-3 .9.9-3 7.6-7.6Z"
              fill="none"
              stroke="currentColor"
              stroke-width="1.4"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <span>Редактировать</span>
        </button>

        <button
          class="task-menu-item task-menu-item--danger"
          type="button"
          role="menuitem"
          data-menu-item
          @click="handleCancelAction"
        >
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" focusable="false">
            <path
              d="M4 4l8 8M12 4l-8 8"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
          <span>Отменить</span>
        </button>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.task-swipe {
  position: relative;
  flex: 0 0 auto;
  border-radius: 12px;
  transition: transform 220ms ease, opacity 220ms ease;
}

.task-swipe--active {
  overflow: hidden;
}

.task-swipe-backdrop {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  padding: 0 18px;
  border-radius: 12px;
  font-size: 0.78rem;
  font-weight: 800;
  user-select: none;
  pointer-events: none;
  transition: background-color 140ms ease, color 140ms ease;
}

.task-swipe-backdrop--cancel {
  justify-content: flex-end;
  color: var(--urgent-dark);
  background: var(--urgent-soft);
}

.task-swipe-backdrop--cancel.task-swipe-backdrop--ready {
  color: #fff;
  background: var(--urgent);
}

.task-swipe-backdrop--complete {
  justify-content: flex-start;
  color: var(--green-dark);
  background: var(--green-soft);
}

.task-swipe-backdrop--complete.task-swipe-backdrop--ready {
  color: #fff;
  background: var(--green);
}

.swipe-cue {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.swipe-cue-icon {
  font-size: 0.85rem;
  line-height: 1;
}

.task-surface {
  position: relative;
  min-height: 62px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 13px 12px 15px;
  border: 1px solid var(--line);
  border-left: 4px solid #a9b4a5;
  border-radius: 12px;
  background: var(--surface);
  box-shadow: 0 4px 14px rgba(46, 57, 43, 0.05);
  touch-action: pan-y;
  transition:
    transform 180ms cubic-bezier(0.22, 1, 0.36, 1),
    box-shadow 160ms ease,
    border-color 160ms ease;
}

.task-surface--dragging {
  transition: none;
  user-select: none;
}

.task-surface--expandable {
  align-items: flex-start;
  cursor: pointer;
}

.task-surface--current {
  border-color: #aec6b4;
  border-left-color: var(--green);
  box-shadow: 0 7px 20px rgba(47, 107, 78, 0.12);
}

.task-surface--urgent {
  border-color: #efc7bd;
  border-left-color: var(--urgent);
  background: #fffaf8;
}

.task-surface--urgent.task-surface--current {
  box-shadow: 0 7px 20px rgba(197, 75, 50, 0.12);
}

@media (hover: hover) {
  .task-surface--expandable:hover {
    box-shadow: 0 6px 18px rgba(46, 57, 43, 0.09);
  }
}

.task-copy {
  flex: 1 1 auto;
  min-width: 0;
}

.task-name {
  margin: 4px 0 0;
  overflow-wrap: anywhere;
  font-size: 0.88rem;
  font-weight: 650;
  line-height: 1.35;
}

.task-description {
  margin: 5px 0 0;
  color: #545d52;
  font-size: 0.77rem;
  font-weight: 450;
  line-height: 1.42;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.task-description:not(.task-description--expanded) {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow: hidden;
}

.task-link {
  color: var(--green);
  text-decoration: underline;
  text-underline-offset: 2px;
  cursor: pointer;
}

.task-surface--urgent .task-link {
  color: var(--urgent-dark);
}

@media (hover: hover) {
  .task-link:hover {
    color: var(--green-dark);
  }

  .task-surface--urgent .task-link:hover {
    color: var(--urgent);
  }
}

.task-link:focus-visible {
  outline: 2px solid rgba(47, 107, 78, 0.35);
  outline-offset: 1px;
  border-radius: 3px;
}

.task-labels {
  min-height: 16px;
  display: flex;
  gap: 5px;
}

.status-badge,
.priority-badge {
  padding: 2px 6px;
  border-radius: 5px;
  font-size: 0.56rem;
  font-weight: 850;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.status-badge {
  color: var(--green-dark);
  background: var(--green-soft);
}

.priority-badge {
  color: var(--urgent-dark);
  background: var(--urgent-soft);
}

.task-actions {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 6px;
}

.menu-button {
  flex: 0 0 auto;
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 7px;
  color: var(--muted);
  background: transparent;
  cursor: pointer;
  transition: opacity 150ms ease, background-color 150ms ease, border-color 150ms ease, color 150ms ease;
}

.menu-button svg {
  width: 14px;
  height: 14px;
}

@media (hover: hover) {
  .menu-button {
    opacity: 0.36;
  }

  .task-surface:hover .menu-button,
  .task-surface:focus-within .menu-button,
  .menu-button:hover,
  .menu-button:focus-visible,
  .menu-button--open {
    opacity: 1;
  }

  .menu-button:hover:not(:disabled),
  .menu-button--open:not(:disabled) {
    color: var(--ink);
    border-color: var(--line);
    background: #eef2ec;
  }

  .task-surface--urgent .menu-button:hover:not(:disabled),
  .task-surface--urgent .menu-button--open:not(:disabled) {
    border-color: #efc7bd;
    background: var(--urgent-soft);
    color: var(--urgent-dark);
  }
}

@media (hover: none), (pointer: coarse) {
  .menu-button {
    opacity: 0.85;
    border-color: rgba(207, 214, 204, 0.7);
    background: rgba(243, 245, 241, 0.7);
  }

  .task-surface--urgent .menu-button {
    border-color: rgba(239, 199, 189, 0.8);
    background: rgba(251, 232, 226, 0.55);
  }
}

.menu-button:disabled {
  cursor: wait;
  opacity: 0.4;
}

.complete-button {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 7px 10px;
  border: 1px solid #b9cdbd;
  border-radius: 8px;
  color: var(--green-dark);
  background: #edf5ef;
  font-size: 0.7rem;
  font-weight: 800;
  cursor: pointer;
  transition: background-color 150ms ease, transform 150ms ease;
}

@media (hover: hover) {
  .complete-button:hover {
    transform: translateY(-1px);
    background: #dfeee3;
  }
}

.complete-button:disabled {
  cursor: wait;
  opacity: 0.55;
  transform: none;
}

@media (pointer: coarse) {
  .menu-button {
    width: 36px;
    height: 36px;
  }

  .complete-button {
    min-height: 40px;
    padding-inline: 14px;
  }
}

@media (max-width: 520px) {
  .task-surface {
    align-items: flex-start;
  }
}
</style>

<style>
.task-menu {
  position: fixed;
  z-index: 50;
  min-width: 158px;
  padding: 5px;
  border: 1px solid var(--line);
  border-radius: 11px;
  background: var(--surface);
  box-shadow: 0 14px 36px rgba(30, 37, 29, 0.18);
  display: flex;
  flex-direction: column;
  gap: 2px;
  animation: task-menu-in 120ms ease-out;
}

.task-menu-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 0;
  border-radius: 7px;
  color: var(--ink);
  background: transparent;
  font-size: 0.78rem;
  font-weight: 650;
  text-align: left;
  cursor: pointer;
}

@media (hover: hover) {
  .task-menu-item:hover {
    background: rgba(47, 107, 78, 0.08);
  }

  .task-menu-item--danger:hover {
    color: var(--urgent-dark);
    background: var(--urgent-soft);
  }
}

.task-menu-item--danger {
  color: var(--urgent-dark);
}

@media (pointer: coarse) {
  .task-menu-item {
    min-height: 40px;
    padding: 9px 11px;
  }
}

@keyframes task-menu-in {
  from {
    transform: translateY(-4px);
    opacity: 0;
  }
}
</style>
