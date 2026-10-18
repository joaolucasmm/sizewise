<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { getTopFiles, type TreeNode } from "../api";
import { openMenu, showToast } from "../actions";
import { formatBytes, formatDate } from "../format";
import ItemIcon from "./ItemIcon.vue";
import RowActions from "./RowActions.vue";

const { t, locale } = useI18n();
const files = ref<TreeNode[] | null>(null);

function folderOf(node: TreeNode): string {
  return node.path.slice(0, Math.max(0, node.path.length - node.name.length - 1));
}

onMounted(async () => {
  try {
    files.value = await getTopFiles();
  } catch (error) {
    files.value = [];
    showToast(t("errors.generic", { message: String(error) }), "error");
  }
});
</script>

<template>
  <div class="top">
    <p v-if="files === null" class="muted pad">{{ t("tree.loading") }}</p>
    <p v-else-if="files.length === 0" class="muted pad">{{ t("top.empty") }}</p>
    <table v-else>
      <thead>
        <tr>
          <th class="num">{{ t("top.rank") }}</th>
          <th>{{ t("top.name") }}</th>
          <th>{{ t("top.path") }}</th>
          <th class="num">{{ t("top.size") }}</th>
          <th>{{ t("top.modified") }}</th>
          <th />
        </tr>
      </thead>
      <tbody>
        <tr v-for="(file, i) in files" :key="file.id" @contextmenu="openMenu($event, file)">
          <td class="num muted">{{ i + 1 }}</td>
          <td class="top-name" :title="file.name"><ItemIcon kind="file" /> {{ file.name }}</td>
          <td class="top-path muted" :title="file.path">{{ folderOf(file) }}</td>
          <td class="num">{{ formatBytes(file.size, locale) }}</td>
          <td class="muted nowrap">{{ formatDate(file.modified, locale) }}</td>
          <td><RowActions :node="file" /></td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
