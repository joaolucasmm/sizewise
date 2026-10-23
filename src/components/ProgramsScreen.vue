<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  listInstalledPrograms,
  measureProgram,
  onUninstallFinished,
  openFolder,
  startUninstall,
  type Program,
  type UninstallFailure,
} from "../api";
import { showToast } from "../actions";
import { askConfirm } from "../confirm";
import { formatBytes, formatDay } from "../format";
import { useSort } from "../sort";
import SortHeader from "./SortHeader.vue";

const { t, locale } = useI18n();
const programs = ref<Program[]>([]);
const loading = ref(true);
const loadError = ref<string | null>(null);
const search = ref("");
/** Sizes measured on request, by program id. */
const measured = reactive(new Map<string, number>());
const measuring = reactive(new Set<string>());
const uninstalling = reactive(new Set<string>());

async function load() {
  try {
    programs.value = await listInstalledPrograms();
    loadError.value = null;
  } catch (error) {
    loadError.value = t("errors.generic", { message: String(error) });
  } finally {
    loading.value = false;
  }
}

let unlisten: (() => void) | null = null;
onMounted(async () => {
  unlisten = await onUninstallFinished(async (finished) => {
    const name = programs.value.find((p) => p.id === finished.id)?.name ?? "";
    uninstalling.delete(finished.id);
    await load();
    const removed = !programs.value.some((p) => p.id === finished.id);
    showToast(removed ? t("programs.removed", { name }) : t("programs.uninstallerClosed", { name }));
  });
  await load();
});
onUnmounted(() => unlisten?.());

function size(program: Program): number | null {
  return program.sizeBytes ?? measured.get(program.id) ?? null;
}

const filtered = computed(() => {
  const query = search.value.trim().toLocaleLowerCase();
  if (!query) return programs.value;
  return programs.value.filter((p) =>
    [p.name, p.publisher ?? ""].some((text) => text.toLocaleLowerCase().includes(query)),
  );
});

const { sorted, column, direction, sortBy } = useSort(
  filtered,
  {
    name: (p) => p.name,
    publisher: (p) => p.publisher,
    size,
    date: (p) => p.installDate,
  },
  "size",
);

const totalKnown = computed(() => programs.value.reduce((sum, p) => sum + (size(p) ?? 0), 0));

async function measure(program: Program) {
  measuring.add(program.id);
  try {
    measured.set(program.id, await measureProgram(program.id));
  } catch (error) {
    showToast(t("programs.measureFailed", { name: program.name, message: String(error) }), "error");
  } finally {
    measuring.delete(program.id);
  }
}

async function reveal(program: Program) {
  if (!program.installLocation) return;
  try {
    await openFolder(program.installLocation);
  } catch (error) {
    showToast(t("programs.openFolderFailed", { message: String(error) }), "error");
  }
}

function failureMessage(failure: UninstallFailure, name: string): string {
  switch (failure.reason) {
    case "cancelled":
      return t("programs.cancelled", { name });
    case "uninstallerMissing":
      return t("programs.uninstallerMissing", { name });
    case "notFound":
      return t("programs.notFound", { name });
    default:
      return t("programs.uninstallFailed", { name, message: failure.message });
  }
}

async function uninstall(program: Program) {
  const confirmed = await askConfirm({
    title: t("programs.confirmTitle", { name: program.name }),
    message: [t("programs.confirmBody", { name: program.name }), t("programs.confirmUndo")],
    confirmLabel: t("programs.uninstall"),
  });
  if (!confirmed) return;
  uninstalling.add(program.id);
  try {
    await startUninstall(program.id);
    showToast(t("programs.started", { name: program.name }));
  } catch (error) {
    uninstalling.delete(program.id);
    const failure = error as UninstallFailure;
    const kind = failure.reason === "cancelled" ? "info" : "error";
    showToast(failure.reason ? failureMessage(failure, program.name) : String(error), kind);
  }
}
</script>

<template>
  <section class="diagnosis diagnosis-fill">
    <h1>{{ t("programs.title") }}</h1>
    <p class="muted">{{ t("programs.intro") }}</p>

    <div class="toolbar">
      <input
        v-model="search"
        class="search"
        type="search"
        :placeholder="t('programs.search')"
        :aria-label="t('programs.search')"
      />
      <span v-if="!loading" class="muted small">
        {{ t("programs.count", { count: programs.length, size: formatBytes(totalKnown, locale) }) }}
      </span>
    </div>

    <div v-if="loadError" class="banner error" role="alert">{{ loadError }}</div>
    <p v-else-if="loading" class="muted">{{ t("common.loading") }}</p>
    <p v-else-if="sorted.length === 0" class="muted">{{ t("programs.empty") }}</p>
    <div v-else class="table-card table-fill">
      <table class="data-table">
        <thead>
          <tr>
            <SortHeader :label="t('programs.name')" :active="column === 'name'" :direction="direction" @sort="sortBy('name', 'asc')" />
            <SortHeader :label="t('programs.publisher')" :active="column === 'publisher'" :direction="direction" @sort="sortBy('publisher', 'asc')" />
            <th class="col-version">{{ t("programs.version") }}</th>
            <SortHeader :label="t('programs.installed')" :active="column === 'date'" :direction="direction" @sort="sortBy('date')" />
            <SortHeader :label="t('programs.size')" numeric :active="column === 'size'" :direction="direction" @sort="sortBy('size')" />
            <th class="actions-cell wide"><span class="visually-hidden">{{ t("common.actions") }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="program in sorted" :key="program.id">
            <td :title="program.name">{{ program.name }}</td>
            <td :title="program.publisher ?? ''">{{ program.publisher ?? "—" }}</td>
            <td class="col-version" :title="program.version ?? ''">{{ program.version ?? "—" }}</td>
            <td>{{ formatDay(program.installDate, locale) }}</td>
            <td class="num">
              <template v-if="program.sizeBytes !== null">{{ formatBytes(program.sizeBytes, locale) }}</template>
              <span v-else-if="measured.has(program.id)" :title="t('programs.measuredHint')">
                {{ formatBytes(measured.get(program.id)!, locale) }}*
              </span>
              <template v-else>
                <span class="muted">{{ t("common.unknown") }}</span>
                <button
                  v-if="program.installLocation"
                  class="link small measure"
                  :disabled="measuring.has(program.id)"
                  @click="measure(program)"
                >
                  {{ measuring.has(program.id) ? t("programs.measuring") : t("programs.measure") }}
                </button>
              </template>
            </td>
            <td class="actions-cell wide">
              <button
                class="icon-button"
                :disabled="!program.installLocation"
                :title="t('programs.openFolder')"
                :aria-label="t('programs.openFolder')"
                @click="reveal(program)"
              >
                <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M1.5 3h4l1.5 1.5h7.5v9h-13v-10.5Zm1.5 3v6h10v-6H3Z" /></svg>
              </button>
              <button class="small-button danger" :disabled="uninstalling.has(program.id)" @click="uninstall(program)">
                {{ uninstalling.has(program.id) ? t("programs.uninstalling") : t("programs.uninstall") }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
