<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { listDrives, type Drive } from "../api";
import { formatBytes } from "../format";

defineProps<{ error: string | null }>();
const emit = defineEmits<{ scan: [target: string]; dismiss: [] }>();

const { t, locale } = useI18n();
const drives = ref<Drive[] | null>(null);
const loadError = ref<string | null>(null);

onMounted(async () => {
  try {
    drives.value = await listDrives();
  } catch (error) {
    loadError.value = t("errors.generic", { message: String(error) });
    drives.value = [];
  }
});

function usedRatio(drive: Drive): number {
  return drive.totalBytes > 0 ? (drive.totalBytes - drive.freeBytes) / drive.totalBytes : 0;
}

async function chooseFolder() {
  const folder = await open({ directory: true, multiple: false, title: t("start.chooseFolder") });
  if (typeof folder === "string") emit("scan", folder);
}
</script>

<template>
  <section class="start">
    <h1>{{ t("start.title") }}</h1>

    <div v-if="error || loadError" class="banner error" role="alert">
      <span>{{ error ?? loadError }}</span>
      <button v-if="error" class="link" @click="emit('dismiss')">{{ t("errors.dismiss") }}</button>
    </div>

    <h2>{{ t("start.drives") }}</h2>
    <p v-if="drives === null" class="muted">{{ t("start.loadingDrives") }}</p>
    <p v-else-if="drives.length === 0" class="muted">{{ t("start.noDrives") }}</p>
    <div v-else class="drives">
      <button v-for="drive in drives" :key="drive.root" class="drive" @click="emit('scan', drive.root)">
        <svg class="drive-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 5h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Zm0 2v10h16V7H4Zm12 6.5a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0Z" />
        </svg>
        <span class="drive-body">
          <span class="drive-name">
            {{ drive.label || t("start.localDisk") }} ({{ drive.letter }}:)
            <span class="muted small">{{ drive.fileSystem }}</span>
          </span>
          <span class="meter" aria-hidden="true">
            <span class="meter-fill" :style="{ width: `${usedRatio(drive) * 100}%` }" />
          </span>
          <span class="muted small">
            {{ t("start.free", { free: formatBytes(drive.freeBytes, locale), total: formatBytes(drive.totalBytes, locale) }) }}
          </span>
        </span>
      </button>
    </div>

    <h2>{{ t("start.folder") }}</h2>
    <div class="folder-pick">
      <button class="primary" @click="chooseFolder">{{ t("start.chooseFolder") }}</button>
      <span class="muted small">{{ t("start.folderHint") }}</span>
    </div>

    <p class="muted small read-only">{{ t("start.readOnly") }}</p>
  </section>
</template>
