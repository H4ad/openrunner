<script setup lang="ts">
import { computed, ref } from "vue";
import type { ProcessStatus } from "../../types";
import { useProcessesStore } from "../../stores/processes";
import { useConfigStore } from "../../stores/config";

const props = defineProps<{
  projectId: string;
  groupId: string;
  status: ProcessStatus;
  autoRestart: boolean;
}>();

const processes = useProcessesStore();
const config = useConfigStore();
const loading = ref(false);

const isRunning = computed(() => props.status === "running");

async function start() {
  loading.value = true;
  try {
    await processes.startProcess(props.groupId, props.projectId);
  } finally {
    loading.value = false;
  }
}

async function stop() {
  loading.value = true;
  try {
    await processes.stopProcess(props.projectId);
  } finally {
    loading.value = false;
  }
}

async function restart() {
  loading.value = true;
  try {
    await processes.restartProcess(props.groupId, props.projectId);
  } finally {
    loading.value = false;
  }
}

async function toggleAutoRestart() {
  await config.updateProject(props.groupId, props.projectId, {
    autoRestart: !props.autoRestart,
  });
}
</script>

<template>
  <div class="flex items-center gap-2 flex-wrap">
    <button
      v-if="!isRunning"
      class="px-3 py-1.5 text-xs rounded bg-green-600 text-white hover:bg-green-500 disabled:opacity-50 transition-colors"
      :disabled="loading"
      @click="start"
    >
      Start
    </button>
    <button
      v-if="isRunning"
      class="px-3 py-1.5 text-xs rounded bg-red-600 text-white hover:bg-red-500 disabled:opacity-50 transition-colors"
      :disabled="loading"
      @click="stop"
    >
      Stop
    </button>
    <button
      class="px-3 py-1.5 text-xs rounded bg-gray-600 text-white hover:bg-gray-500 disabled:opacity-50 transition-colors"
      :disabled="loading"
      @click="restart"
    >
      Restart
    </button>

    <div class="ml-auto flex items-center gap-2">
      <span class="text-xs text-gray-400">Auto-restart</span>
      <button
        class="relative w-8 h-4 rounded-full transition-colors"
        :class="props.autoRestart ? 'bg-blue-600' : 'bg-gray-600'"
        @click="toggleAutoRestart"
      >
        <span
          class="absolute top-0.5 w-3 h-3 rounded-full bg-white transition-transform"
          :class="props.autoRestart ? 'translate-x-4' : 'translate-x-0.5'"
        ></span>
      </button>
    </div>
  </div>
</template>
