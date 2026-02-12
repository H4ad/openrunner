<script setup lang="ts">
import { computed } from "vue";
import { useConfigStore } from "../../stores/config";
import { useProcessesStore } from "../../stores/processes";
import { useUiStore } from "../../stores/ui";
import type { Project } from "../../types";
import { formatBytes, formatCpu } from "../../utils/formatters";
import StatusBadge from "../shared/StatusBadge.vue";
import { cn } from "@/lib/utils";

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

const projectStatus = computed(() => {
  const info = process.statuses.get(props.project.id);
  return {
    status: info?.status ?? "stopped",
    cpuUsage: info?.cpuUsage ?? 0,
    memoryUsage: info?.memoryUsage ?? 0,
  };
});

const status = computed(() => projectStatus.value.status);
const isRunning = computed(() => projectStatus.value.status === "running");
const formattedCpu = computed(() => formatCpu(projectStatus.value.cpuUsage));
const formattedMemory = computed(() => formatBytes(projectStatus.value.memoryUsage));

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
    :class="cn(
      'w-full text-left px-3 py-1.5 text-sm rounded flex items-baseline justify-between transition-colors cursor-pointer',
      isSelected
        ? isActive
          ? 'bg-accent text-foreground'
          : 'bg-accent/50 text-foreground'
        : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground'
    )"
    @click="handleClick"
    @contextmenu.prevent="handleContextMenu"
  >
    <span class="truncate">{{ props.project.name }}</span>
    <div class="flex items-baseline gap-1.5 flex-shrink-0 ml-1">
      <span
        v-if="isRunning"
        class="text-[10px] text-muted-foreground font-mono"
      >{{ formattedCpu }} {{ formattedMemory }}</span>
      <StatusBadge :status="status" />
    </div>
  </button>
</template>
