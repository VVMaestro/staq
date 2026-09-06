<script setup lang="ts">
import type { ResizeDirection } from "../useWindowFrame";

defineProps<{ maximized: boolean }>();
const emit = defineEmits<{
  minimize: [];
  toggleMaximize: [];
  close: [];
  resize: [direction: ResizeDirection];
}>();

const directions: ResizeDirection[] = [
  "North", "South", "East", "West",
  "NorthEast", "NorthWest", "SouthEast", "SouthWest",
];

function resize(event: PointerEvent, direction: ResizeDirection) {
  if (event.button === 0 && event.isPrimary) emit("resize", direction);
}
</script>

<template>
  <header class="window-titlebar" aria-label="Управление окном">
    <div class="window-drag-region" data-tauri-drag-region>
      <img
        class="window-app-icon"
        src="/staq-icon.png"
        alt=""
        width="16"
        height="16"
        draggable="false"
        data-tauri-drag-region
      />
      <span data-tauri-drag-region>STAQ</span>
    </div>
    <div class="window-buttons">
      <button type="button" aria-label="Свернуть" title="Свернуть" @click="emit('minimize')">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5h10" /></svg>
      </button>
      <button
        type="button"
        :aria-label="maximized ? 'Восстановить' : 'Развернуть'"
        :title="maximized ? 'Восстановить' : 'Развернуть'"
        @click="emit('toggleMaximize')"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path v-if="maximized" d="M5.5 5.5v-3h8v8h-3m-8-5h8v8h-8z" />
          <path v-else d="M3.5 3.5h9v9h-9z" />
        </svg>
      </button>
      <button class="window-close" type="button" aria-label="Закрыть окно" title="Закрыть окно" @click="emit('close')">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 4 8 8m0-8-8 8" /></svg>
      </button>
    </div>
  </header>
  <template v-if="!maximized">
    <div
      v-for="direction in directions"
      :key="direction"
      class="window-resize"
      :class="`window-resize--${direction}`"
      aria-hidden="true"
      @pointerdown.prevent="resize($event, direction)"
    ></div>
  </template>
</template>

<style scoped>
.window-titlebar {
  display: flex;
  flex: 0 0 32px;
  height: 32px;
  user-select: none;
  background: #f2f4ee;
}

.window-drag-region {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
  padding-left: 14px;
  color: var(--muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.14em;
}

.window-app-icon {
  flex-shrink: 0;
  object-fit: contain;
}

.window-buttons { display: flex; }
.window-buttons button {
  display: grid;
  place-items: center;
  width: 44px;
  height: 32px;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
}
.window-buttons button:hover { background: #dfe5da; }
.window-buttons .window-close:hover { color: white; background: #c54b32; }
.window-buttons button:focus-visible { outline-offset: -4px; }
.window-buttons svg { width: 16px; height: 16px; fill: none; stroke: currentColor; }

.window-resize { position: absolute; z-index: 10; touch-action: none; }
.window-resize--North, .window-resize--South { left: 12px; right: 12px; height: 5px; cursor: ns-resize; }
.window-resize--North { top: 0; }
.window-resize--South { bottom: 0; }
.window-resize--East, .window-resize--West { top: 12px; bottom: 12px; width: 5px; cursor: ew-resize; }
.window-resize--East { right: 0; }
.window-resize--West { left: 0; }
.window-resize--NorthEast, .window-resize--NorthWest,
.window-resize--SouthEast, .window-resize--SouthWest { width: 12px; height: 12px; }
.window-resize--NorthEast { top: 0; right: 0; cursor: nesw-resize; }
.window-resize--NorthWest { top: 0; left: 0; cursor: nwse-resize; }
.window-resize--SouthEast { bottom: 0; right: 0; cursor: nwse-resize; }
.window-resize--SouthWest { bottom: 0; left: 0; cursor: nesw-resize; }
</style>
