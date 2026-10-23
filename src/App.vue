<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { cancelScan, onScanEvents, startScan, type Progress, type ScanFailure, type Summary } from "./api";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import ContextMenu from "./components/ContextMenu.vue";
import DisksScreen from "./components/DisksScreen.vue";
import LanguageSwitcher from "./components/LanguageSwitcher.vue";
import OverviewScreen from "./components/OverviewScreen.vue";
import ProcessesScreen from "./components/ProcessesScreen.vue";
import ProgramsScreen from "./components/ProgramsScreen.vue";
import ResultsScreen from "./components/ResultsScreen.vue";
import ScanScreen from "./components/ScanScreen.vue";
import StartScreen from "./components/StartScreen.vue";
import StartupScreen from "./components/StartupScreen.vue";
import { toast } from "./actions";
import { SECTIONS, type Section } from "./sections";

type Screen =
  | { name: "start"; error: string | null }
  | { name: "scanning"; target: string; progress: Progress | null; startedAt: number; cancelling: boolean }
  | { name: "results"; summary: Summary };

const { t } = useI18n();
const section = ref<Section>("disk");
const screen = ref<Screen>({ name: "start", error: null });

// Scan ids grow by one per Scan. While `start_scan` is in flight its id is
// not known yet, so any newer id is taken to be the Scan being started.
let currentScanId = 0;
let starting = false;

function isCurrent(scanId: number): boolean {
  if (scanId === currentScanId) return true;
  if (starting && scanId > currentScanId) {
    currentScanId = scanId;
    return true;
  }
  return false;
}

function failureMessage(failure: ScanFailure): string {
  const values = { target: failure.target, message: failure.message };
  switch (failure.reason) {
    case "notFound":
      return t("errors.notFound", values);
    case "notAFolder":
      return t("errors.notAFolder", values);
    case "unreadable":
      return t("errors.unreadable", values);
    default:
      return t("errors.internal", values);
  }
}

let unlisten: (() => void) | null = null;

onMounted(async () => {
  unlisten = await onScanEvents({
    progress(progress) {
      if (isCurrent(progress.scanId) && screen.value.name === "scanning") {
        screen.value.progress = progress;
      }
    },
    completed(summary) {
      if (isCurrent(summary.scanId)) screen.value = { name: "results", summary };
    },
    cancelled(summary) {
      if (isCurrent(summary.scanId)) screen.value = { name: "results", summary };
    },
    failed(failure) {
      if (isCurrent(failure.scanId)) screen.value = { name: "start", error: failureMessage(failure) };
    },
  });
});

onUnmounted(() => unlisten?.());

async function begin(target: string) {
  section.value = "disk";
  screen.value = { name: "scanning", target, progress: null, startedAt: Date.now(), cancelling: false };
  starting = true;
  try {
    currentScanId = await startScan(target);
  } catch (error) {
    screen.value = { name: "start", error: t("errors.generic", { message: String(error) }) };
  } finally {
    starting = false;
  }
}

async function cancel() {
  if (screen.value.name !== "scanning") return;
  screen.value.cancelling = true;
  await cancelScan();
}

function newScan() {
  screen.value = { name: "start", error: null };
}
</script>

<template>
  <div class="app">
    <header class="topbar">
      <div class="brand">
        <svg class="logo" viewBox="0 0 32 32" aria-hidden="true">
          <rect x="3" y="18" width="6" height="11" rx="1.5" />
          <rect x="13" y="10" width="6" height="19" rx="1.5" />
          <rect x="23" y="3" width="6" height="26" rx="1.5" />
        </svg>
        <span class="brand-name">Sizewise</span>
        <span class="brand-tagline">{{ t("app.tagline") }}</span>
      </div>
      <LanguageSwitcher />
    </header>

    <div class="body">
      <nav class="sidenav" :aria-label="t('nav.label')">
        <button
          v-for="item in SECTIONS"
          :key="item"
          :class="{ active: section === item }"
          :aria-current="section === item ? 'page' : undefined"
          @click="section = item"
        >
          {{ t(`nav.${item}`) }}
        </button>
      </nav>

      <main class="content">
        <!-- The disk space section stays mounted so a Scan and its results survive switching sections. -->
        <div v-show="section === 'disk'" class="section-disk">
          <StartScreen v-if="screen.name === 'start'" :error="screen.error" @scan="begin" @dismiss="screen.error = null" />
          <ScanScreen
            v-else-if="screen.name === 'scanning'"
            :target="screen.target"
            :progress="screen.progress"
            :started-at="screen.startedAt"
            :cancelling="screen.cancelling"
            @cancel="cancel"
          />
          <ResultsScreen v-else :key="screen.summary.scanId" :summary="screen.summary" @new-scan="newScan" />
        </div>
        <OverviewScreen v-if="section === 'overview'" @open="section = $event" @scan="begin" />
        <StartupScreen v-else-if="section === 'startup'" />
        <ProcessesScreen v-else-if="section === 'processes'" />
        <ProgramsScreen v-else-if="section === 'programs'" />
        <DisksScreen v-else-if="section === 'disks'" @scan="begin" />
      </main>
    </div>

    <ContextMenu />
    <ConfirmDialog />
    <div v-if="toast.message" class="toast" :class="toast.kind" role="status">{{ toast.message }}</div>
  </div>
</template>
