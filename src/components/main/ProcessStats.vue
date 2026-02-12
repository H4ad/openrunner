<script setup lang="ts">
import { computed } from "vue";
import { Badge } from "@/components/ui/badge";
import { DesktopIcon, LayersIcon } from "@radix-icons/vue";
import { formatBytes } from "../../utils/formatters";

const props = defineProps<{
  cpuUsage: number;
  memoryUsage: number;
}>();

const formattedMemory = computed(() => formatBytes(props.memoryUsage));
</script>

<template>
  <div class="flex items-center gap-4 text-xs text-muted-foreground">
    <div class="flex items-center gap-1.5">
      <DesktopIcon class="h-3.5 w-3.5" />
      <span>CPU: {{ props.cpuUsage.toFixed(1) }}%</span>
    </div>
    <div class="flex items-center gap-1.5">
      <LayersIcon class="h-3.5 w-3.5" />
      <span>MEM: {{ formattedMemory }}</span>
    </div>
  </div>
</template>
