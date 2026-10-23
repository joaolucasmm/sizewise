<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { listPhysicalDisks, listVolumes, type Availability, type PhysicalDisk, type Volume } from "../api";
import { formatBytes } from "../format";
import UnavailableNote from "./UnavailableNote.vue";
import VolumeCard from "./VolumeCard.vue";

const emit = defineEmits<{ scan: [target: string] }>();

const { t, locale } = useI18n();
const disks = ref<Availability<PhysicalDisk[]> | null>(null);
const volumes = ref<Volume[] | null>(null);
const loadError = ref<string | null>(null);

onMounted(async () => {
  try {
    [disks.value, volumes.value] = await Promise.all([listPhysicalDisks(), listVolumes()]);
  } catch (error) {
    loadError.value = t("errors.generic", { message: String(error) });
  }
});

const problems = computed(() => disks.value?.value?.filter((d) => d.hasProblem) ?? []);
</script>

<template>
  <section class="diagnosis">
    <h1>{{ t("disks.title") }}</h1>
    <p class="muted">{{ t("disks.intro") }}</p>
    <div v-if="loadError" class="banner error" role="alert">{{ loadError }}</div>

    <div v-for="disk in problems" :key="disk.id" class="banner error-strong" role="alert">
      <strong>{{ t("disks.problemTitle", { model: disk.model || disk.id }) }}</strong>
      <span>{{ t("disks.problemBody") }}</span>
    </div>

    <h2>{{ t("disks.physical") }}</h2>
    <p v-if="disks === null && !loadError" class="muted">{{ t("common.loading") }}</p>
    <UnavailableNote v-else-if="disks?.unavailable" :unavailable="disks.unavailable" />
    <div v-else-if="disks?.value" class="cards">
      <div v-for="disk in disks.value" :key="disk.id" class="card" :class="{ problem: disk.hasProblem }">
        <div class="card-title">{{ disk.model || t("disks.unnamed") }}</div>
        <div class="muted small">
          {{ t(`disks.media.${disk.mediaType}`) }} · {{ disk.busType }} · {{ formatBytes(disk.sizeBytes, locale) }}
        </div>
        <div class="health">
          <span class="badge" :class="disk.health === 'healthy' ? 'ok' : disk.health === 'unknown' ? 'off' : 'danger'">
            {{ t(`disks.health.${disk.health}`) }}
          </span>
        </div>
      </div>
    </div>

    <h2>{{ t("disks.volumes") }}</h2>
    <p v-if="volumes === null && !loadError" class="muted">{{ t("common.loading") }}</p>
    <div v-else-if="volumes" class="cards">
      <VolumeCard v-for="volume in volumes" :key="volume.root" :volume="volume" @scan="emit('scan', volume.root)" />
    </div>
  </section>
</template>
