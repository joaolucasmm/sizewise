<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { answerConfirm, confirmState } from "../confirm";

const { t } = useI18n();
const cancelButton = ref<HTMLButtonElement | null>(null);

// Cancel gets the focus, so Enter never confirms by accident.
watch(
  () => confirmState.request,
  async (request) => {
    if (!request) return;
    await nextTick();
    cancelButton.value?.focus();
  },
);

function onKey(event: KeyboardEvent) {
  if (event.key === "Escape" && confirmState.request) answerConfirm(false);
}

onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div v-if="confirmState.request" class="dialog-backdrop" @click.self="answerConfirm(false)">
    <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title">
      <h2 id="confirm-title">{{ confirmState.request.title }}</h2>
      <p v-for="(paragraph, i) in confirmState.request.message" :key="i">{{ paragraph }}</p>
      <div class="dialog-actions">
        <button ref="cancelButton" @click="answerConfirm(false)">{{ t("common.cancel") }}</button>
        <button class="danger" @click="answerConfirm(true)">{{ confirmState.request.confirmLabel }}</button>
      </div>
    </div>
  </div>
</template>
