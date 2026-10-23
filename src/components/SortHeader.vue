<script setup lang="ts">
import type { Direction } from "../sort";

defineProps<{ label: string; active: boolean; direction: Direction; numeric?: boolean }>();
const emit = defineEmits<{ sort: [] }>();
</script>

<template>
  <th :class="{ num: numeric }" :aria-sort="active ? (direction === 'asc' ? 'ascending' : 'descending') : 'none'">
    <button class="sort-button" :class="{ active }" @click="emit('sort')">
      {{ label }}
      <span v-if="active" aria-hidden="true">{{ direction === "asc" ? "▲" : "▼" }}</span>
    </button>
  </th>
</template>
