<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  getStartupDurations,
  listStartupItems,
  revealPath,
  type Availability,
  type StartupDuration,
  type StartupItem,
  type StartupSource,
} from "../api";
import { showToast } from "../actions";
import { formatDate, formatDuration } from "../format";
import { useSort } from "../sort";
import SortHeader from "./SortHeader.vue";
import UnavailableNote from "./UnavailableNote.vue";

const { t, locale } = useI18n();
const items = ref<StartupItem[]>([]);
const loading = ref(true);
const loadError = ref<string | null>(null);
const durations = ref<Availability<StartupDuration[]> | null>(null);

onMounted(async () => {
  try {
    const [list, measured] = await Promise.all([listStartupItems(), getStartupDurations()]);
    items.value = list;
    durations.value = measured;
  } catch (error) {
    loadError.value = t("errors.generic", { message: String(error) });
  } finally {
    loading.value = false;
  }
});

function sourceLabel(source: StartupSource): string {
  switch (source.kind) {
    case "registryRun":
      return t(source.scope === "currentUser" ? "startup.source.runUser" : "startup.source.runMachine");
    case "startupFolder":
      return t(source.scope === "currentUser" ? "startup.source.folderUser" : "startup.source.folderMachine");
    case "storeApp":
      return t("startup.source.storeApp");
    case "scheduledTask":
      return t("startup.source.task");
  }
}

const enabledCount = computed(() => items.value.filter((i) => i.enabled).length);

const { sorted, column, direction, sortBy } = useSort(
  items,
  {
    name: (i) => i.name,
    publisher: (i) => i.publisher,
    source: (i) => sourceLabel(i.source),
    // Enabled first when sorting descending.
    state: (i) => (i.enabled ? 1 : 0),
  },
  "state",
);

async function reveal(item: StartupItem) {
  if (!item.target) return;
  try {
    await revealPath(item.target);
  } catch (error) {
    showToast(t("actions.revealFailed", { message: String(error) }), "error");
  }
}
</script>

<template>
  <section class="diagnosis">
    <h1>{{ t("startup.title") }}</h1>
    <p class="muted">{{ t("startup.intro") }}</p>

    <div class="banner info">{{ t("startup.viewOnly") }}</div>

    <h2>{{ t("startup.durations") }}</h2>
    <p v-if="durations === null && loading" class="muted">{{ t("common.loading") }}</p>
    <UnavailableNote v-else-if="durations?.unavailable" :unavailable="durations.unavailable" />
    <p v-else-if="durations?.value && durations.value.length === 0" class="muted">
      {{ t("startup.noDurations") }}
    </p>
    <dl v-else-if="durations?.value" class="stats">
      <div v-for="d in durations.value.slice(0, 5)" :key="d.recordedAt">
        <dt>{{ formatDate(d.recordedAt, locale) }}</dt>
        <dd>{{ formatDuration(d.bootMs, locale) }}</dd>
        <dd class="muted small">
          {{ t("startup.untilDesktop", { time: formatDuration(d.mainPathMs, locale) }) }}
        </dd>
      </div>
    </dl>

    <h2>{{ t("startup.items", { enabled: enabledCount, total: items.length }) }}</h2>
    <p class="muted small">{{ t("startup.noImpact") }}</p>
    <div v-if="loadError" class="banner error" role="alert">{{ loadError }}</div>
    <p v-else-if="loading" class="muted">{{ t("common.loading") }}</p>
    <p v-else-if="items.length === 0" class="muted">{{ t("startup.empty") }}</p>
    <div v-else class="table-card">
      <table class="data-table">
        <thead>
          <tr>
            <SortHeader :label="t('startup.name')" :active="column === 'name'" :direction="direction" @sort="sortBy('name', 'asc')" />
            <SortHeader :label="t('startup.publisher')" :active="column === 'publisher'" :direction="direction" @sort="sortBy('publisher', 'asc')" />
            <SortHeader :label="t('startup.sourceColumn')" :active="column === 'source'" :direction="direction" @sort="sortBy('source', 'asc')" />
            <SortHeader :label="t('startup.state')" :active="column === 'state'" :direction="direction" @sort="sortBy('state')" />
            <th>{{ t("startup.command") }}</th>
            <th class="actions-cell"><span class="visually-hidden">{{ t("common.actions") }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in sorted" :key="`${item.source.kind}:${item.name}:${item.command}`">
            <td :title="item.name">{{ item.name }}</td>
            <td :title="item.publisher ?? ''">{{ item.publisher ?? "—" }}</td>
            <td :title="item.source.kind === 'scheduledTask' ? item.source.path : ''">{{ sourceLabel(item.source) }}</td>
            <td>
              <span class="badge" :class="item.enabled ? 'ok' : 'off'">
                {{ item.enabled ? t("startup.enabled") : t("startup.disabled") }}
              </span>
              <span v-if="item.broken" class="badge warning" :title="t('startup.brokenHint')">{{ t("startup.broken") }}</span>
            </td>
            <td class="mono" :title="item.command">{{ item.command }}</td>
            <td class="actions-cell">
              <button
                class="icon-button"
                :disabled="!item.target || item.broken"
                :title="t('actions.openLocation')"
                :aria-label="t('actions.openLocation')"
                @click="reveal(item)"
              >
                <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M1.5 3h4l1.5 1.5h7.5v9h-13v-10.5Zm1.5 3v6h10v-6H3Z" /></svg>
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
