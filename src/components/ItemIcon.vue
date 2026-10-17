<script setup lang="ts">
defineProps<{ kind: "folder" | "file" }>();
</script>

<template>
  <svg v-if="kind === 'folder'" class="item-icon folder" viewBox="0 0 16 16" aria-hidden="true">
    <path d="M1.5 3.5A1.5 1.5 0 0 1 3 2h3.4l1.5 1.5H13A1.5 1.5 0 0 1 14.5 5v7.5A1.5 1.5 0 0 1 13 14H3a1.5 1.5 0 0 1-1.5-1.5v-9Z" />
  </svg>
  <svg v-else class="item-icon file" viewBox="0 0 16 16" aria-hidden="true">
    <path d="M4 1.5h5L12.5 5v9a.5.5 0 0 1-.5.5H4a.5.5 0 0 1-.5-.5V2a.5.5 0 0 1 .5-.5Zm5 0V5h3.5" />
  </svg>
</template>
