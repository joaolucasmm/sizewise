<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { ProcessInfo } from "../api";

defineProps<{ process: ProcessInfo; busy: boolean; hint: string }>();
const emit = defineEmits<{ end: [] }>();
const { t } = useI18n();
</script>

<template>
  <!-- Protected processes get a label with the reason instead of a button. -->
  <span v-if="process.protection" class="badge off" :title="hint">{{ t("processes.protected") }}</span>
  <button v-else class="small-button danger" :disabled="busy" @click="emit('end')">
    {{ t("processes.endTask") }}
  </button>
</template>
