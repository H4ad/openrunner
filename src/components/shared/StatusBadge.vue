<script setup lang="ts">
import type { ProcessStatus } from "../../types";
import { Badge } from "@/components/ui/badge";

const props = defineProps<{
  status?: ProcessStatus;
}>();

const variantMap: Record<string, "default" | "secondary" | "destructive" | "outline"> = {
  running: "default",
  stopping: "outline",
  stopped: "secondary",
  errored: "destructive",
};

const colorMap: Record<string, string> = {
  running: "bg-emerald-500",
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
  <Badge :variant="variantMap[props.status ?? 'stopped']" class="gap-1.5">
    <span
      class="w-2 h-2 rounded-full"
      :class="colorMap[props.status ?? 'stopped']"
    />
    {{ labelMap[props.status ?? "stopped"] }}
  </Badge>
</template>
