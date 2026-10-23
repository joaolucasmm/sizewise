<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { Volume } from "../api";
import { formatBytes, formatPercent } from "../format";

const props = defineProps<{ volume: Volume }>();
const emit = defineEmits<{ scan: [] }>();
const { t, locale } = useI18n();

const used = () =>
  props.volume.totalBytes > 0 ? (props.volume.totalBytes - props.volume.freeBytes) / props.volume.totalBytes : 0;
</script>

<template>
  <div class="card" :class="{ problem: volume.lowSpace }">
    <div class="card-title">{{ volume.label || t("start.localDisk") }} ({{ volume.letter }}:)</div>
    <span class="meter" :class="{ low: volume.lowSpace }" aria-hidden="true">
      <span class="meter-fill" :style="{ width: `${used() * 100}%` }" />
    </span>
    <div class="muted small">
      {{ t("start.free", { free: formatBytes(volume.freeBytes, locale), total: formatBytes(volume.totalBytes, locale) }) }}
      ({{ formatPercent(volume.freeBytes, volume.totalBytes, locale) }})
    </div>
    <div v-if="volume.lowSpace" class="low-space">
      <span>{{ t("disks.lowSpace") }}</span>
      <button class="small-button" @click="emit('scan')">{{ t("disks.scanDrive") }}</button>
    </div>
  </div>
</template>
