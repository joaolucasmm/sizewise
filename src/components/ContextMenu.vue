<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { closeMenu, copyPath, menu, reveal } from "../actions";

const { t } = useI18n();
const element = ref<HTMLElement | null>(null);
const size = ref({ width: 0, height: 0 });

// Keep the menu inside the window when opened near an edge.
const position = computed(() => ({
  left: `${Math.min(menu.x, window.innerWidth - size.value.width - 8)}px`,
  top: `${Math.min(menu.y, window.innerHeight - size.value.height - 8)}px`,
}));

watch(
  () => menu.open,
  async (open) => {
    if (!open) return;
    await nextTick();
    if (element.value) {
      size.value = { width: element.value.offsetWidth, height: element.value.offsetHeight };
      element.value.querySelector("button")?.focus();
    }
  },
);

function onKey(event: KeyboardEvent) {
  if (event.key === "Escape") closeMenu();
}

onMounted(() => {
  window.addEventListener("keydown", onKey);
  window.addEventListener("blur", closeMenu);
  window.addEventListener("resize", closeMenu);
});
onUnmounted(() => {
  window.removeEventListener("keydown", onKey);
  window.removeEventListener("blur", closeMenu);
  window.removeEventListener("resize", closeMenu);
});
</script>

<template>
  <div v-if="menu.open && menu.node" class="menu-backdrop" @click="closeMenu" @contextmenu.prevent="closeMenu">
    <div ref="element" class="menu" role="menu" :style="position" @click.stop>
      <button role="menuitem" @click="reveal(menu.node)">{{ t("actions.reveal") }}</button>
      <button role="menuitem" @click="copyPath(menu.node)">{{ t("actions.copyPath") }}</button>
    </div>
  </div>
</template>
