<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { DEFAULT_STAQ_ID, staqLabel, staqTaskCount, type StaqSnapshot } from "../staq";

const props = defineProps<{
  staqs: StaqSnapshot[];
  selectedId: number;
  disabled: boolean;
}>();

const emit = defineEmits<{
  select: [id: number];
  create: [];
  delete: [staq: StaqSnapshot];
}>();

const isOpen = ref(false);
const rootRef = ref<HTMLElement | null>(null);
const buttonRef = ref<HTMLButtonElement | null>(null);
const menuRef = ref<HTMLElement | null>(null);

const currentLabel = computed(() => {
  const current = props.staqs.find((staq) => staq.id === props.selectedId);
  return current ? staqLabel(current) : "…";
});

function menuItems() {
  return Array.from(menuRef.value?.querySelectorAll<HTMLElement>("[data-menu-item]") ?? []);
}

async function openMenu(focus: "selected" | "first" | "last" = "selected") {
  if (props.disabled) return;
  isOpen.value = true;
  await nextTick();
  const items = menuItems();
  const target =
    focus === "last"
      ? items[items.length - 1]
      : focus === "selected"
        ? menuRef.value?.querySelector<HTMLElement>("[aria-checked='true']")
        : items[0];
  (target ?? items[0])?.focus();
}

function closeMenu(restoreFocus = true) {
  if (!isOpen.value) return;
  isOpen.value = false;
  if (restoreFocus) buttonRef.value?.focus();
}

function toggleMenu() {
  if (isOpen.value) closeMenu();
  else void openMenu();
}

function handleOutsidePointer(event: PointerEvent) {
  if (!rootRef.value?.contains(event.target as Node)) closeMenu(false);
}

function handleMenuKeydown(event: KeyboardEvent) {
  const items = menuItems();
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
    case "Escape":
      event.preventDefault();
      closeMenu();
      break;
    case "Tab":
      closeMenu(false);
      break;
  }
}

function selectStaq(id: number) {
  closeMenu();
  if (id !== props.selectedId) emit("select", id);
}

function requestCreate() {
  closeMenu(false);
  emit("create");
}

function requestDelete(staq: StaqSnapshot) {
  closeMenu(false);
  emit("delete", staq);
}

watch(isOpen, (open) => {
  if (open) document.addEventListener("pointerdown", handleOutsidePointer);
  else document.removeEventListener("pointerdown", handleOutsidePointer);
});

watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) closeMenu(false);
  },
);

onBeforeUnmount(() => document.removeEventListener("pointerdown", handleOutsidePointer));

defineExpose({ focus: () => buttonRef.value?.focus() });
</script>

<template>
  <div ref="rootRef" class="staq-switcher">
    <button
      ref="buttonRef"
      class="switcher-button"
      type="button"
      aria-haspopup="menu"
      aria-controls="staq-menu"
      :aria-expanded="isOpen"
      :disabled="disabled"
      @click="toggleMenu"
      @keydown.down.prevent="openMenu('first')"
      @keydown.up.prevent="openMenu('last')"
    >
      <span class="switcher-label">{{ currentLabel }}</span>
      <span class="switcher-chevron" :class="{ 'switcher-chevron--open': isOpen }" aria-hidden="true"></span>
    </button>

    <div
      v-if="isOpen"
      id="staq-menu"
      ref="menuRef"
      class="staq-menu"
      role="menu"
      aria-label="Стеки"
      @keydown="handleMenuKeydown"
    >
      <div
        v-for="staq in staqs"
        :key="staq.id"
        class="menu-row"
        :class="{ 'menu-row--selected': staq.id === selectedId }"
        role="none"
      >
        <button
          class="menu-item"
          type="button"
          role="menuitemradio"
          data-menu-item
          :aria-checked="staq.id === selectedId"
          @click="selectStaq(staq.id)"
        >
          <span class="menu-check" aria-hidden="true">{{ staq.id === selectedId ? "✓" : "" }}</span>
          <span class="menu-name">{{ staqLabel(staq) }}</span>
          <span class="menu-count" :aria-label="`задач: ${staqTaskCount(staq)}`">
            {{ staqTaskCount(staq) }}
          </span>
        </button>
        <button
          v-if="staq.id !== DEFAULT_STAQ_ID"
          class="menu-delete"
          type="button"
          role="menuitem"
          data-menu-item
          :aria-label="`Удалить стек ${staqLabel(staq)}`"
          @click="requestDelete(staq)"
        >
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
            <path
              d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5M7 7v4M9 7v4"
              fill="none"
              stroke="currentColor"
              stroke-width="1.3"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>
      </div>

      <div class="menu-separator" role="separator"></div>

      <div class="menu-row" role="none">
        <button
          class="menu-item menu-item--create"
          type="button"
          role="menuitem"
          data-menu-item
          @click="requestCreate"
        >
          <span class="menu-check" aria-hidden="true">+</span>
          <span class="menu-name">Новый стек</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.staq-switcher {
  position: relative;
  z-index: 10;
  min-width: 0;
}

.switcher-button {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.7);
  box-shadow: 0 2px 8px rgba(46, 57, 43, 0.05);
  cursor: pointer;
  transition: background-color 150ms ease, border-color 150ms ease;
}

.switcher-button:hover:not(:disabled),
.switcher-button[aria-expanded="true"] {
  border-color: #b9cdbd;
  background: #fff;
}

.switcher-button:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.switcher-label {
  min-width: 0;
  overflow: hidden;
  font-size: 0.9rem;
  font-weight: 750;
  line-height: 1.3;
  letter-spacing: -0.01em;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.switcher-chevron {
  flex: 0 0 auto;
  width: 8px;
  height: 8px;
  margin-top: -4px;
  border-right: 2px solid var(--muted);
  border-bottom: 2px solid var(--muted);
  transform: rotate(45deg);
  transition: transform 150ms ease, margin 150ms ease;
}

.switcher-chevron--open {
  margin-top: 4px;
  transform: rotate(-135deg);
}

.staq-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  width: max(100%, min(280px, calc(100vw - 40px)));
  max-height: min(340px, calc(100vh - 140px));
  overflow-y: auto;
  padding: 6px;
  border: 1px solid var(--line);
  border-radius: 13px;
  background: var(--surface);
  box-shadow: 0 18px 45px rgba(30, 37, 29, 0.18);
  animation: menu-in 140ms ease-out;
}

.menu-row {
  display: flex;
  align-items: center;
  gap: 2px;
  border-radius: 9px;
}

.menu-row--selected {
  background: var(--green-soft);
}

.menu-item,
.menu-delete {
  border: 0;
  border-radius: 8px;
  background: transparent;
  cursor: pointer;
}

.menu-item {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 9px;
  font-size: 0.8rem;
  font-weight: 650;
  text-align: left;
}

.menu-item:hover,
.menu-delete:hover {
  background: rgba(47, 107, 78, 0.08);
}

.menu-check {
  flex: 0 0 14px;
  color: var(--green);
  font-weight: 850;
  text-align: center;
}

.menu-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.menu-count {
  flex: 0 0 auto;
  min-width: 20px;
  padding: 1px 6px;
  border-radius: 999px;
  color: var(--muted);
  background: rgba(255, 255, 255, 0.8);
  font-size: 0.66rem;
  font-weight: 800;
  text-align: center;
}

.menu-delete {
  flex: 0 0 auto;
  width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  color: var(--muted);
}

.menu-delete:hover {
  color: var(--urgent-dark);
  background: var(--urgent-soft);
}

.menu-separator {
  height: 1px;
  margin: 5px 4px;
  background: var(--line);
}

.menu-item--create {
  color: var(--green-dark);
}

@keyframes menu-in {
  from {
    transform: translateY(-4px);
    opacity: 0;
  }
}
</style>
