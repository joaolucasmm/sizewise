<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { TreeNode } from "../api";
import { openMenu } from "../actions";

defineProps<{ node: TreeNode }>();
const { t } = useI18n();
</script>

<template>
  <button
    class="icon-button row-actions"
    :aria-label="t('actions.menu', { name: node.name })"
    :title="t('actions.menu', { name: node.name })"
    @click.stop="openMenu($event, node)"
    @keydown.enter.stop
    @keydown.space.stop
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="3" cy="8" r="1.4" />
      <circle cx="8" cy="8" r="1.4" />
      <circle cx="13" cy="8" r="1.4" />
    </svg>
  </button>
</template>
