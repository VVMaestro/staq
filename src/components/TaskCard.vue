<script setup lang="ts">
defineProps<{
  title: string;
  urgent: boolean;
  current: boolean;
  busy: boolean;
}>();

defineEmits<{
  complete: [];
}>();
</script>

<template>
  <article
    class="task-card"
    :class="{
      'task-card--current': current,
      'task-card--urgent': urgent,
      'task-card--regular': !urgent,
    }"
  >
    <div class="task-copy">
      <div class="task-labels">
        <span v-if="current" class="status-badge">Сейчас</span>
        <span v-if="urgent" class="priority-badge">Срочно</span>
      </div>
      <p>{{ title }}</p>
    </div>

    <button
      v-if="current"
      class="complete-button"
      type="button"
      :aria-label="`Завершить задачу: ${title}`"
      :disabled="busy"
      @click="$emit('complete')"
    >
      <span aria-hidden="true">✓</span>
      Готово
    </button>
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
  gap: 14px;
  padding: 12px 13px 12px 15px;
  border: 1px solid var(--line);
  border-left: 4px solid #a9b4a5;
  border-radius: 12px;
  background: var(--surface);
  box-shadow: 0 4px 14px rgba(46, 57, 43, 0.05);
  transition: transform 220ms ease, opacity 220ms ease, box-shadow 160ms ease;
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

.task-copy {
  min-width: 0;
}

.task-copy p {
  margin: 4px 0 0;
  overflow-wrap: anywhere;
  font-size: 0.88rem;
  font-weight: 650;
  line-height: 1.35;
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

.complete-button:hover {
  transform: translateY(-1px);
  background: #dfeee3;
}

.complete-button:disabled {
  cursor: wait;
  opacity: 0.55;
  transform: none;
}

@media (max-width: 520px) {
  .task-card {
    align-items: flex-start;
  }
}
</style>
