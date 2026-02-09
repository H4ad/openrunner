<script setup lang="ts">
import { computed } from "vue";
import { formatBytes } from "../../utils/formatters";

const props = defineProps<{
  cpuUsage: number;
  memoryUsage: number;
}>();

const formattedMemory = computed(() => formatBytes(props.memoryUsage));
</script>

<template>
  <div class="flex items-center gap-4 text-xs text-gray-400">
    <div class="flex items-center gap-1.5">
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
        />
      </svg>
      <span>CPU: {{ props.cpuUsage.toFixed(1) }}%</span>
    </div>
    <div class="flex items-center gap-1.5">
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
        />
      </svg>
      <span>MEM: {{ formattedMemory }}</span>
    </div>
  </div>
</template>
