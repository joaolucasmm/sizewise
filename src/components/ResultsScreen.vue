<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { getUnreadable, type Summary } from "../api";
import { formatBytes, formatCount, formatDuration } from "../format";
import TopFiles from "./TopFiles.vue";
import TreeView from "./TreeView.vue";

const props = defineProps<{ summary: Summary }>();
const emit = defineEmits<{ "new-scan": [] }>();

const { t, locale } = useI18n();
const tab = ref<"tree" | "top">("tree");
const unreadable = ref<string[] | null>(null);
const showUnreadable = ref(false);

async function toggleUnreadable() {
  showUnreadable.value = !showUnreadable.value;
  if (showUnreadable.value && unreadable.value === null) {
    unreadable.value = await getUnreadable();
  }
}
</script>

<template>
  <section class="results">
    <div class="summary">
      <div class="summary-target">
        <h1 :title="props.summary.target">{{ props.summary.target }}</h1>
        <button @click="emit('new-scan')">{{ t("results.newScan") }}</button>
      </div>
      <dl class="stats">
        <div>
          <dt>{{ t("results.total") }}</dt>
          <dd>{{ formatBytes(summary.totalSize, locale) }}</dd>
        </div>
        <div>
          <dt>{{ t("results.files") }}</dt>
          <dd>{{ formatCount(summary.totalFiles, locale) }}</dd>
        </div>
        <div>
          <dt>{{ t("results.folders") }}</dt>
          <dd>{{ formatCount(summary.totalFolders, locale) }}</dd>
        </div>
        <div>
          <dt>{{ t("results.duration") }}</dt>
          <dd>{{ formatDuration(summary.durationMs, locale) }}</dd>
        </div>
      </dl>
    </div>

    <div v-if="summary.state === 'cancelled'" class="banner warning" role="alert">
      {{ t("results.cancelled") }}
    </div>
    <div v-if="summary.unreadableCount > 0" class="banner warning">
      <div class="banner-row">
        <span>{{ t("results.unreadable", summary.unreadableCount) }}</span>
        <button class="link" @click="toggleUnreadable">
          {{ showUnreadable ? t("results.hideUnreadable") : t("results.showUnreadable") }}
        </button>
      </div>
      <ul v-if="showUnreadable && unreadable" class="unreadable">
        <li v-for="path in unreadable" :key="path">{{ path }}</li>
      </ul>
    </div>

    <div class="tabs" role="tablist">
      <button role="tab" :aria-selected="tab === 'tree'" :class="{ active: tab === 'tree' }" @click="tab = 'tree'">
        {{ t("results.tree") }}
      </button>
      <button role="tab" :aria-selected="tab === 'top'" :class="{ active: tab === 'top' }" @click="tab = 'top'">
        {{ t("results.top") }}
      </button>
    </div>

    <!-- Both stay mounted so switching tabs keeps the tree's expanded state. -->
    <TreeView v-show="tab === 'tree'" :root="summary.root" />
    <TopFiles v-if="tab === 'top'" />
  </section>
</template>
