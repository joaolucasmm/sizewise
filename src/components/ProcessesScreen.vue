<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  endProcess,
  revealPath,
  watchProcesses,
  type EndTaskFailure,
  type ProcessGroup,
  type ProcessInfo,
  type ProcessSnapshot,
} from "../api";
import { showToast } from "../actions";
import { askConfirm } from "../confirm";
import { formatBytes, formatCpu, formatPercent, formatRate } from "../format";
import { useSort } from "../sort";
import EndTaskButton from "./EndTaskButton.vue";
import SortHeader from "./SortHeader.vue";

const { t, locale } = useI18n();
const snapshot = ref<ProcessSnapshot | null>(null);
const groups = computed(() => snapshot.value?.groups ?? []);
const expanded = reactive(new Set<string>());
const ending = reactive(new Set<number>());

let stop: (() => void) | null = null;
onMounted(async () => {
  stop = await watchProcesses((next) => (snapshot.value = next));
});
onUnmounted(() => stop?.());

const { sorted, column, direction, sortBy } = useSort(
  groups,
  {
    name: (g) => g.name,
    cpu: (g) => g.cpuPercent,
    ram: (g) => g.ramBytes,
    read: (g) => g.readBytesPerSec,
    write: (g) => g.writeBytesPerSec,
  },
  "cpu",
);

function toggle(group: ProcessGroup) {
  if (expanded.has(group.key)) expanded.delete(group.key);
  else expanded.add(group.key);
}

function protectionHint(process: ProcessInfo): string {
  if (process.protection === "critical") return t("processes.criticalHint");
  if (process.protection === "ownProcess") return t("processes.ownHint");
  return "";
}

async function reveal(path: string | null) {
  if (!path) return;
  try {
    await revealPath(path);
  } catch (error) {
    showToast(t("actions.revealFailed", { message: String(error) }), "error");
  }
}

function failureMessage(failure: EndTaskFailure, name: string): string {
  switch (failure.reason) {
    case "critical":
      return t("processes.criticalHint");
    case "ownProcess":
      return t("processes.ownHint");
    case "accessDenied":
      return t("processes.accessDenied", { name });
    case "alreadyExited":
      return t("processes.alreadyExited", { name });
    default:
      return t("processes.endFailed", { name, message: failure.message });
  }
}

async function end(process: ProcessInfo) {
  if (process.protection) return;
  const confirmed = await askConfirm({
    title: t("processes.confirmTitle", { name: process.name }),
    message: [t("processes.confirmBody", { name: process.name, pid: process.pid }), t("common.cantUndo")],
    confirmLabel: t("processes.endTask"),
  });
  if (!confirmed) return;
  ending.add(process.pid);
  try {
    await endProcess(process.pid);
    showToast(t("processes.ended", { name: process.name }));
  } catch (error) {
    const failure = error as EndTaskFailure;
    const kind = failure.reason === "alreadyExited" ? "info" : "error";
    showToast(failure.reason ? failureMessage(failure, process.name) : String(error), kind);
  } finally {
    ending.delete(process.pid);
  }
}
</script>

<template>
  <section class="diagnosis diagnosis-fill">
    <h1>{{ t("processes.title") }}</h1>
    <p class="muted">{{ t("processes.intro") }}</p>

    <dl v-if="snapshot" class="stats">
      <div>
        <dt>{{ t("processes.cpu") }}</dt>
        <dd>{{ formatCpu(snapshot.totals.cpuPercent, locale) }}</dd>
      </div>
      <div>
        <dt>{{ t("processes.ram") }}</dt>
        <dd>{{ formatBytes(snapshot.totals.ramUsedBytes, locale) }}</dd>
        <dd class="muted small">
          {{ t("processes.ramOf", {
            total: formatBytes(snapshot.totals.ramTotalBytes, locale),
            percent: formatPercent(snapshot.totals.ramUsedBytes, snapshot.totals.ramTotalBytes, locale),
          }) }}
        </dd>
      </div>
      <div>
        <dt>{{ t("processes.read") }}</dt>
        <dd>{{ formatRate(snapshot.totals.readBytesPerSec, locale) }}</dd>
      </div>
      <div>
        <dt>{{ t("processes.write") }}</dt>
        <dd>{{ formatRate(snapshot.totals.writeBytesPerSec, locale) }}</dd>
      </div>
    </dl>
    <p class="muted small">{{ t("processes.ioNote") }}</p>

    <p v-if="!snapshot" class="muted">{{ t("common.loading") }}</p>
    <div v-else class="table-card table-fill">
      <table class="data-table">
        <thead>
          <tr>
            <SortHeader :label="t('processes.name')" :active="column === 'name'" :direction="direction" @sort="sortBy('name', 'asc')" />
            <SortHeader :label="t('processes.cpu')" numeric :active="column === 'cpu'" :direction="direction" @sort="sortBy('cpu')" />
            <SortHeader :label="t('processes.ram')" numeric :active="column === 'ram'" :direction="direction" @sort="sortBy('ram')" />
            <SortHeader :label="t('processes.read')" numeric :active="column === 'read'" :direction="direction" @sort="sortBy('read')" />
            <SortHeader :label="t('processes.write')" numeric :active="column === 'write'" :direction="direction" @sort="sortBy('write')" />
            <th class="actions-cell wide"><span class="visually-hidden">{{ t("common.actions") }}</span></th>
          </tr>
        </thead>
        <tbody>
          <template v-for="group in sorted" :key="group.key">
            <tr>
              <td :title="group.path ?? group.name">
                <button
                  v-if="group.processes.length > 1"
                  class="chevron-button"
                  :aria-expanded="expanded.has(group.key)"
                  :aria-label="t('processes.expand', { name: group.name })"
                  @click="toggle(group)"
                >
                  <span class="chevron" :class="{ open: expanded.has(group.key) }">
                    <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6 4l4 4-4 4" /></svg>
                  </span>
                </button>
                <span v-else class="chevron-spacer" />
                {{ group.name }}
                <span v-if="group.processes.length > 1" class="muted small">({{ group.processes.length }})</span>
              </td>
              <td class="num">{{ formatCpu(group.cpuPercent, locale) }}</td>
              <td class="num">{{ group.ramBytes === null ? "—" : formatBytes(group.ramBytes, locale) }}</td>
              <td class="num">{{ formatRate(group.readBytesPerSec, locale) }}</td>
              <td class="num">{{ formatRate(group.writeBytesPerSec, locale) }}</td>
              <td class="actions-cell wide">
                <button
                  class="icon-button"
                  :disabled="!group.path"
                  :title="t('actions.openLocation')"
                  :aria-label="t('actions.openLocation')"
                  @click="reveal(group.path)"
                >
                  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M1.5 3h4l1.5 1.5h7.5v9h-13v-10.5Zm1.5 3v6h10v-6H3Z" /></svg>
                </button>
                <EndTaskButton
                  v-if="group.processes.length === 1"
                  :process="group.processes[0]"
                  :busy="ending.has(group.processes[0].pid)"
                  :hint="protectionHint(group.processes[0])"
                  @end="end(group.processes[0])"
                />
              </td>
            </tr>
            <template v-if="group.processes.length > 1 && expanded.has(group.key)">
              <tr v-for="process in group.processes" :key="process.pid" class="child-row">
                <td :title="process.path ?? process.name">
                  {{ process.name }} <span class="muted small">{{ t("processes.pid", { pid: process.pid }) }}</span>
                </td>
                <td class="num">{{ formatCpu(process.cpuPercent, locale) }}</td>
                <td class="num">{{ process.ramBytes === null ? "—" : formatBytes(process.ramBytes, locale) }}</td>
                <td class="num">{{ formatRate(process.readBytesPerSec, locale) }}</td>
                <td class="num">{{ formatRate(process.writeBytesPerSec, locale) }}</td>
                <td class="actions-cell wide">
                  <EndTaskButton
                    :process="process"
                    :busy="ending.has(process.pid)"
                    :hint="protectionHint(process)"
                    @end="end(process)"
                  />
                </td>
              </tr>
            </template>
          </template>
        </tbody>
      </table>
    </div>
  </section>
</template>
