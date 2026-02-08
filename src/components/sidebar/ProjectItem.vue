<script setup lang="ts">
import type { Project } from "../../types";
import { useProcessesStore } from "../../stores/processes";
import { useUiStore } from "../../stores/ui";
import StatusBadge from "../shared/StatusBadge.vue";
import { computed } from "vue";

const props = defineProps<{
  project: Project;
  groupId: string;
}>();

const ui = useUiStore();
const processes = useProcessesStore();

const processInfo = computed(() => processes.getStatus(props.project.id));
const status = computed(
  () => processInfo.value?.status ?? "stopped",
);
const isSelected = computed(
  () =>
    ui.selectedProjectId === props.project.id &&
    ui.selectedGroupId === props.groupId,
);
const isRunning = computed(() => status.value === "running");

const formattedCpu = computed(() => {
  const cpu = processInfo.value?.cpuUsage ?? 0;
  return cpu.toFixed(0) + "%";
});

const formattedMemory = computed(() => {
  const bytes = processInfo.value?.memoryUsage ?? 0;
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}K`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(0)}M`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}G`;
});
</script>

<template>
  <button
    class="w-full text-left px-3 py-1.5 text-sm rounded flex items-center justify-between transition-colors"
    :class="
      isSelected
        ? 'bg-gray-700 text-gray-100'
        : 'text-gray-400 hover:bg-gray-700/50 hover:text-gray-300'
    "
    @click="ui.selectProject(props.groupId, props.project.id)"
  >
    <span class="truncate">{{ props.project.name }}</span>
    <div class="flex items-center gap-1.5 flex-shrink-0 ml-1">
      <span
        v-if="isRunning"
        class="text-[10px] text-gray-500 font-mono"
      >{{ formattedCpu }} {{ formattedMemory }}</span>
      <StatusBadge :status="status" />
    </div>
  </button>
</template>
