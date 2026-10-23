<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { Unavailable } from "../api";

defineProps<{ unavailable: Unavailable }>();
const { t } = useI18n();
</script>

<template>
  <p class="unavailable-note">
    <template v-if="unavailable.reason === 'needsAdministrator'">{{ t("common.needsAdmin") }}</template>
    <template v-else>{{ t("common.unavailable", { message: unavailable.message }) }}</template>
  </p>
</template>
