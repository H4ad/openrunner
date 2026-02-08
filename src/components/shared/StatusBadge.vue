<script setup lang="ts">
import type { ProcessStatus } from "../../types";

const props = defineProps<{
  status?: ProcessStatus;
}>();

const colorMap: Record<string, string> = {
  running: "bg-green-500",
  stopping: "bg-yellow-500",
  stopped: "bg-gray-500",
  errored: "bg-red-500",
};

const labelMap: Record<string, string> = {
  running: "Running",
  stopping: "Stopping...",
  stopped: "Stopped",
  errored: "Errored",
};
</script>

<template>
  <span class="inline-flex items-center gap-1.5 text-xs">
    <span
      class="w-2 h-2 rounded-full"
      :class="colorMap[props.status ?? 'stopped']"
    ></span>
    <span class="text-gray-400">{{ labelMap[props.status ?? "stopped"] }}</span>
  </span>
</template>
