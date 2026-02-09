<script setup lang="ts">
import { computed } from "vue";
import { useConfigStore } from "../../stores/config";
import { useProcessesStore } from "../../stores/processes";
import { useUiStore } from "../../stores/ui";
import type { Project } from "../../types";
import { formatBytes, formatCpu } from "../../utils/formatters";
import StatusBadge from "../shared/StatusBadge.vue";

const props = defineProps<{
  project: Project;
  groupId: string;
}>();

const emit = defineEmits<{
  contextmenu: [e: MouseEvent];
}>();

const ui = useUiStore();
const process = useProcessesStore();
const config = useConfigStore();

const isSelected = computed(() =>
  ui.isProjectSelected(props.groupId, props.project.id),
);

const isActive = computed(
  () =>
    ui.selectedGroupId === props.groupId &&
    ui.selectedProjectId === props.project.id,
);

const status = computed(() => process.statuses.get(props.project.id)?.status ?? "stopped");

const isRunning = computed(() => status.value === "running");

const cpuUsage = computed(() => process.statuses.get(props.project.id)?.cpuUsage ?? 0);

const memoryUsage = computed(() => process.statuses.get(props.project.id)?.memoryUsage ?? 0);

const formattedCpu = computed(() => formatCpu(cpuUsage.value));
const formattedMemory = computed(() => formatBytes(memoryUsage.value));

const projectIndex = computed(() => {
  const group = config.groups.find((g) => g.id === props.groupId);
  if (!group) return -1;
  return group.projects.findIndex((p) => p.id === props.project.id);
});

function handleClick(e: MouseEvent) {
  if (e.shiftKey && ui.selectedProjectId) {
    ui.selectProject(props.groupId, props.project.id, true);
  } else {
    ui.selectProject(props.groupId, props.project.id, false);
  }
}

function handleContextMenu(e: MouseEvent) {
  emit("contextmenu", e);
}
</script>

<template>
  <button
    class="w-full text-left px-3 py-1.5 text-sm rounded flex items-center justify-between transition-colors"
    :class="[
      isSelected
        ? isActive
          ? 'bg-gray-600 text-gray-100'
          : 'bg-gray-700/50 text-gray-100'
        : 'text-gray-400 hover:bg-gray-700/50 hover:text-gray-300',
      { 'ring-1 ring-gray-500': isActive },
    ]"
    @click="handleClick"
    @contextmenu.prevent="handleContextMenu"
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
