<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { useI18n } from "vue-i18n";
import { getChildren, type TreeNode } from "../api";
import { openMenu, showToast } from "../actions";
import { formatBytes, formatCount, formatPercent } from "../format";
import ItemIcon from "./ItemIcon.vue";
import RowActions from "./RowActions.vue";

const props = defineProps<{ root: TreeNode }>();
const { t, locale } = useI18n();

const ROW_HEIGHT = 32;
const OVERSCAN = 12;
const PAGE_SIZE = 500;

type NodeRow = { type: "node"; node: TreeNode; depth: number; parentSize: number; expanded: boolean; loading: boolean };
type MoreRow = { type: "more"; parentId: number; depth: number; parentSize: number; loaded: number; total: number; loading: boolean };
type Row = NodeRow | MoreRow;

// The tree is flattened into the rows currently visible when scrolling;
// only the slice inside the viewport is rendered.
const rows = shallowRef<Row[]>([]);
const viewport = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const viewportHeight = ref(0);
const loadingRoot = ref(true);

const visible = computed(() => {
  const start = Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - OVERSCAN);
  const end = Math.min(rows.value.length, Math.ceil((scrollTop.value + viewportHeight.value) / ROW_HEIGHT) + OVERSCAN);
  return rows.value.slice(start, end).map((row, i) => ({ row, index: start + i }));
});

async function fetchRows(parentId: number, parentSize: number, depth: number, offset: number): Promise<Row[]> {
  const page = await getChildren(parentId, offset, PAGE_SIZE);
  const out: Row[] = page.items.map((node) => ({ type: "node", node, depth, parentSize, expanded: false, loading: false }));
  const loaded = offset + page.items.length;
  if (loaded < page.total) {
    out.push({ type: "more", parentId, depth, parentSize, loaded, total: page.total, loading: false });
  }
  return out;
}

function replaceRow(index: number, row: Row, inserted: Row[] = [], removeAfter = 0) {
  const next = rows.value.slice();
  next.splice(index, 1 + removeAfter, row, ...inserted);
  rows.value = next;
}

function indexOfNode(id: number): number {
  return rows.value.findIndex((r) => r.type === "node" && r.node.id === id);
}

async function toggle(row: NodeRow) {
  if (row.node.kind !== "folder" || row.node.childCount === 0 || row.loading) return;
  const index = indexOfNode(row.node.id);
  if (row.expanded) {
    let end = index + 1;
    while (end < rows.value.length && rows.value[end].depth > row.depth) end++;
    replaceRow(index, { ...row, expanded: false }, [], end - index - 1);
    return;
  }
  replaceRow(index, { ...row, loading: true });
  try {
    const children = await fetchRows(row.node.id, row.node.size, row.depth + 1, 0);
    replaceRow(indexOfNode(row.node.id), { ...row, expanded: true, loading: false }, children);
  } catch (error) {
    replaceRow(indexOfNode(row.node.id), { ...row, loading: false });
    showToast(t("errors.generic", { message: String(error) }), "error");
  }
}

async function loadMore(row: MoreRow) {
  if (row.loading) return;
  const marked = { ...row, loading: true };
  replaceRow(rows.value.indexOf(row), marked);
  try {
    const more = await fetchRows(row.parentId, row.parentSize, row.depth, row.loaded);
    const index = rows.value.indexOf(marked);
    const next = rows.value.slice();
    next.splice(index, 1, ...more);
    rows.value = next;
  } catch (error) {
    replaceRow(rows.value.indexOf(marked), row);
    showToast(t("errors.generic", { message: String(error) }), "error");
  }
}

function onKey(event: KeyboardEvent, row: NodeRow) {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    toggle(row);
  } else if (event.key === "ArrowRight" && !row.expanded) {
    toggle(row);
  } else if (event.key === "ArrowLeft" && row.expanded) {
    toggle(row);
  }
}

function share(row: NodeRow): number {
  return row.parentSize > 0 ? row.node.size / row.parentSize : 0;
}

let observer: ResizeObserver | null = null;

onMounted(async () => {
  if (viewport.value) {
    observer = new ResizeObserver(([entry]) => (viewportHeight.value = entry.contentRect.height));
    observer.observe(viewport.value);
  }
  try {
    rows.value = await fetchRows(props.root.id, props.root.size, 0, 0);
  } catch (error) {
    showToast(t("errors.generic", { message: String(error) }), "error");
  } finally {
    loadingRoot.value = false;
  }
});

onUnmounted(() => observer?.disconnect());
</script>

<template>
  <div class="tree">
    <div class="tree-header row-grid">
      <span>{{ t("tree.name") }}</span>
      <span />
      <span class="num">{{ t("tree.percent") }}</span>
      <span class="num">{{ t("tree.size") }}</span>
      <span>{{ t("tree.contents") }}</span>
      <span />
    </div>
    <div ref="viewport" class="tree-viewport" role="tree" @scroll="scrollTop = ($event.target as HTMLElement).scrollTop">
      <p v-if="loadingRoot" class="muted pad">{{ t("tree.loading") }}</p>
      <p v-else-if="rows.length === 0" class="muted pad">{{ t("tree.empty") }}</p>
      <div class="tree-spacer" :style="{ height: `${rows.length * ROW_HEIGHT}px` }">
        <template v-for="{ row, index } in visible" :key="row.type === 'node' ? `n${row.node.id}` : `m${row.parentId}-${row.loaded}`">
          <div
            v-if="row.type === 'node'"
            class="tree-row row-grid"
            role="treeitem"
            tabindex="0"
            :aria-level="row.depth + 1"
            :aria-expanded="row.node.kind === 'folder' && row.node.childCount > 0 ? row.expanded : undefined"
            :style="{ transform: `translateY(${index * ROW_HEIGHT}px)` }"
            @click="toggle(row)"
            @keydown="onKey($event, row)"
            @contextmenu="openMenu($event, row.node)"
          >
            <span class="name-cell" :style="{ paddingLeft: `${row.depth * 18}px` }" :title="row.node.path">
              <span class="chevron" :class="{ open: row.expanded, hidden: row.node.kind !== 'folder' || row.node.childCount === 0 }">
                <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6 3.5 10.5 8 6 12.5" /></svg>
              </span>
              <ItemIcon :kind="row.node.kind" />
              <span class="name">{{ row.node.name }}</span>
              <span v-if="row.loading" class="muted small">{{ t("tree.loading") }}</span>
            </span>
            <span class="bar" aria-hidden="true"><span class="bar-fill" :style="{ width: `${share(row) * 100}%` }" /></span>
            <span class="num muted">{{ formatPercent(row.node.size, row.parentSize, locale) }}</span>
            <span class="num">{{ formatBytes(row.node.size, locale) }}</span>
            <span class="muted small contents">
              <template v-if="row.node.kind === 'folder'">
                {{ t("tree.files", { n: formatCount(row.node.files, locale) }, row.node.files) }} ·
                {{ t("tree.folders", { n: formatCount(row.node.folders, locale) }, row.node.folders) }}
              </template>
            </span>
            <RowActions :node="row.node" />
          </div>
          <div
            v-else
            class="tree-row more-row"
            :style="{ transform: `translateY(${index * ROW_HEIGHT}px)`, paddingLeft: `${row.depth * 18 + 30}px` }"
          >
            <button class="link" :disabled="row.loading" @click="loadMore(row)">
              {{ row.loading ? t("tree.loading") : t("tree.showMore", { count: row.total - row.loaded }) }}
            </button>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
