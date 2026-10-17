<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { Progress } from "../api";
import { formatBytes, formatCount, formatDuration } from "../format";

const props = defineProps<{
  target: string;
  progress: Progress | null;
  startedAt: number;
  cancelling: boolean;
}>();
const emit = defineEmits<{ cancel: [] }>();

const { t, locale } = useI18n();

// Ticks on its own so the clock keeps moving between progress events.
const now = ref(Date.now());
let timer = 0;
onMounted(() => (timer = window.setInterval(() => (now.value = Date.now()), 200)));
onUnmounted(() => window.clearInterval(timer));
</script>

<template>
  <section class="scanning">
    <div class="spinner" aria-hidden="true" />
    <h1>{{ t("scan.title") }}</h1>
    <p class="target" :title="target">{{ target }}</p>

    <dl class="stats">
      <div>
        <dt>{{ t("scan.files") }}</dt>
        <dd>{{ formatCount(props.progress?.files ?? 0, locale) }}</dd>
      </div>
      <div>
        <dt>{{ t("scan.size") }}</dt>
        <dd>{{ formatBytes(props.progress?.bytes ?? 0, locale) }}</dd>
      </div>
      <div>
        <dt>{{ t("scan.elapsed") }}</dt>
        <dd>{{ formatDuration(Math.max(0, now - startedAt), locale) }}</dd>
      </div>
    </dl>

    <div class="current">
      <span class="muted small">{{ t("scan.currentFolder") }}</span>
      <span class="current-path" :title="progress?.currentFolder">{{ progress?.currentFolder ?? t("scan.starting") }}</span>
    </div>

    <button class="danger" :disabled="cancelling" @click="emit('cancel')">
      {{ cancelling ? t("scan.cancelling") : t("scan.cancel") }}
    </button>
  </section>
</template>
