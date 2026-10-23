<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { getOverview, watchProcesses, type Overview, type ProcessSnapshot } from "../api";
import { formatBytes, formatCpu, formatDate, formatDuration, formatPercent } from "../format";
import type { Section } from "../sections";
import UnavailableNote from "./UnavailableNote.vue";
import VolumeCard from "./VolumeCard.vue";

const emit = defineEmits<{ open: [section: Section]; scan: [target: string] }>();

const { t, locale } = useI18n();
const overview = ref<Overview | null>(null);
const loadError = ref<string | null>(null);
const snapshot = ref<ProcessSnapshot | null>(null);

let stop: (() => void) | null = null;
onMounted(async () => {
  stop = await watchProcesses((next) => (snapshot.value = next));
  try {
    overview.value = await getOverview();
  } catch (error) {
    loadError.value = t("errors.generic", { message: String(error) });
  }
});
onUnmounted(() => stop?.());

const topCpu = computed(() =>
  [...(snapshot.value?.groups ?? [])]
    .filter((g) => g.cpuPercent !== null)
    .sort((a, b) => (b.cpuPercent ?? 0) - (a.cpuPercent ?? 0))
    .slice(0, 3),
);
const topRam = computed(() =>
  [...(snapshot.value?.groups ?? [])]
    .filter((g) => g.ramBytes !== null)
    .sort((a, b) => (b.ramBytes ?? 0) - (a.ramBytes ?? 0))
    .slice(0, 3),
);
const diskProblems = computed(() => overview.value?.disks.value?.filter((d) => d.hasProblem) ?? []);
</script>

<template>
  <section class="diagnosis">
    <h1>{{ t("overview.title") }}</h1>
    <p class="muted">{{ t("overview.intro") }}</p>
    <div v-if="loadError" class="banner error" role="alert">{{ loadError }}</div>

    <div class="overview-grid">
      <article class="card">
        <header class="card-header">
          <h2>{{ t("overview.startup") }}</h2>
          <button class="link small" @click="emit('open', 'startup')">{{ t("overview.details") }}</button>
        </header>
        <template v-if="overview">
          <p class="big">{{ overview.enabledStartupItems }}</p>
          <p class="muted small">{{ t("overview.enabledItems", overview.enabledStartupItems) }}</p>
          <UnavailableNote v-if="overview.lastStartup.unavailable" :unavailable="overview.lastStartup.unavailable" />
          <p v-else-if="overview.lastStartup.value" class="small">
            {{ t("overview.lastStartup", {
              time: formatDuration(overview.lastStartup.value.bootMs, locale),
              date: formatDate(overview.lastStartup.value.recordedAt, locale),
            }) }}
          </p>
          <p v-else class="muted small">{{ t("startup.noDurations") }}</p>
        </template>
        <p v-else class="muted">{{ t("common.loading") }}</p>
      </article>

      <article class="card">
        <header class="card-header">
          <h2>{{ t("overview.cpu") }}</h2>
          <button class="link small" @click="emit('open', 'processes')">{{ t("overview.details") }}</button>
        </header>
        <template v-if="snapshot">
          <p class="big">{{ formatCpu(snapshot.totals.cpuPercent, locale) }}</p>
          <ol class="top-list">
            <li v-for="group in topCpu" :key="group.key">
              <span class="name">{{ group.name }}</span>
              <span class="num">{{ formatCpu(group.cpuPercent, locale) }}</span>
            </li>
          </ol>
        </template>
        <p v-else class="muted">{{ t("common.loading") }}</p>
      </article>

      <article class="card">
        <header class="card-header">
          <h2>{{ t("overview.ram") }}</h2>
          <button class="link small" @click="emit('open', 'processes')">{{ t("overview.details") }}</button>
        </header>
        <template v-if="snapshot">
          <p class="big">{{ formatPercent(snapshot.totals.ramUsedBytes, snapshot.totals.ramTotalBytes, locale) }}</p>
          <p class="muted small">
            {{ t("overview.ramUsed", {
              used: formatBytes(snapshot.totals.ramUsedBytes, locale),
              total: formatBytes(snapshot.totals.ramTotalBytes, locale),
            }) }}
          </p>
          <ol class="top-list">
            <li v-for="group in topRam" :key="group.key">
              <span class="name">{{ group.name }}</span>
              <span class="num">{{ formatBytes(group.ramBytes ?? 0, locale) }}</span>
            </li>
          </ol>
        </template>
        <p v-else class="muted">{{ t("common.loading") }}</p>
      </article>

      <article class="card" :class="{ problem: diskProblems.length > 0 }">
        <header class="card-header">
          <h2>{{ t("overview.diskHealth") }}</h2>
          <button class="link small" @click="emit('open', 'disks')">{{ t("overview.details") }}</button>
        </header>
        <template v-if="overview">
          <UnavailableNote v-if="overview.disks.unavailable" :unavailable="overview.disks.unavailable" />
          <template v-else-if="diskProblems.length > 0">
            <p v-for="disk in diskProblems" :key="disk.id" class="danger-text">
              {{ t("disks.problemTitle", { model: disk.model || disk.id }) }}
            </p>
            <p class="small">{{ t("disks.problemBody") }}</p>
          </template>
          <p v-else class="small">{{ t("overview.disksFine", overview.disks.value?.length ?? 0) }}</p>
        </template>
        <p v-else class="muted">{{ t("common.loading") }}</p>
      </article>
    </div>

    <h2>{{ t("overview.space") }}</h2>
    <div v-if="overview" class="cards">
      <VolumeCard v-for="volume in overview.volumes" :key="volume.root" :volume="volume" @scan="emit('scan', volume.root)" />
    </div>
  </section>
</template>
