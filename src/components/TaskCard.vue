<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { parseDescriptionSegments } from "../taskDescription";

const props = defineProps<{
  name: string;
  description: string;
  urgent: boolean;
  current: boolean;
  busy: boolean;
}>();

const emit = defineEmits<{
  complete: [];
  edit: [];
  openLink: [href: string];
}>();

const expanded = ref(false);
const hasDescription = computed(() => props.description.length > 0);
const isExpanded = computed(() => hasDescription.value && expanded.value);
const segments = computed(() => parseDescriptionSegments(props.description));

watch(hasDescription, (value) => {
  if (!value) expanded.value = false;
});

function toggleExpanded() {
  if (!hasDescription.value) return;
  expanded.value = !expanded.value;
}

function handleCardClick(event: MouseEvent) {
  if (!hasDescription.value) return;
  if (event.target instanceof Element && event.target.closest("a, button")) return;
  const selection = window.getSelection();
  if (selection && !selection.isCollapsed) return;
  toggleExpanded();
}
</script>

<template>
  <article
    class="task-card"
    :class="{
      'task-card--current': current,
      'task-card--urgent': urgent,
      'task-card--regular': !urgent,
      'task-card--expandable': hasDescription,
      'task-card--expanded': isExpanded,
    }"
    @click="handleCardClick"
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
        class="edit-button"
        type="button"
        :aria-label="`Редактировать задачу: ${name}`"
        title="Редактировать"
        :disabled="busy"
        @click.stop="emit('edit')"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
          <path
            d="M11.6 2.4a1.5 1.5 0 0 1 2.1 2.1L6.1 12.1l-3 .9.9-3 7.6-7.6Z"
            fill="none"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
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
</template>

<style scoped>
.task-card {
  position: relative;
  flex: 0 0 auto;
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
  transition: transform 220ms ease, opacity 220ms ease, box-shadow 160ms ease, border-color 160ms ease;
}

.task-card--expandable {
  align-items: flex-start;
  cursor: pointer;
}

.task-card--current {
  border-color: #aec6b4;
  border-left-color: var(--green);
  box-shadow: 0 7px 20px rgba(47, 107, 78, 0.12);
}

.task-card--urgent {
  border-color: #efc7bd;
  border-left-color: var(--urgent);
  background: #fffaf8;
}

.task-card--urgent.task-card--current {
  box-shadow: 0 7px 20px rgba(197, 75, 50, 0.12);
}

@media (hover: hover) {
  .task-card--expandable:hover {
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

.task-card--urgent .task-link {
  color: var(--urgent-dark);
}

@media (hover: hover) {
  .task-link:hover {
    color: var(--green-dark);
  }

  .task-card--urgent .task-link:hover {
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

.edit-button {
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

.edit-button svg {
  width: 14px;
  height: 14px;
}

@media (hover: hover) {
  .edit-button {
    opacity: 0.36;
  }

  .task-card:hover .edit-button,
  .task-card:focus-within .edit-button,
  .edit-button:hover,
  .edit-button:focus-visible {
    opacity: 1;
  }

  .edit-button:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--line);
    background: #eef2ec;
  }

  .task-card--urgent .edit-button:hover:not(:disabled) {
    border-color: #efc7bd;
    background: var(--urgent-soft);
    color: var(--urgent-dark);
  }
}

@media (hover: none), (pointer: coarse) {
  .edit-button {
    opacity: 0.85;
    border-color: rgba(207, 214, 204, 0.7);
    background: rgba(243, 245, 241, 0.7);
  }

  .task-card--urgent .edit-button {
    border-color: rgba(239, 199, 189, 0.8);
    background: rgba(251, 232, 226, 0.55);
  }
}

.edit-button:disabled {
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
  .edit-button {
    width: 36px;
    height: 36px;
  }

  .complete-button {
    min-height: 40px;
    padding-inline: 14px;
  }
}

@media (max-width: 520px) {
  .task-card {
    align-items: flex-start;
  }
}
</style>
