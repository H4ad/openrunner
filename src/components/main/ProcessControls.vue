<script setup lang="ts">
import { computed, ref } from "vue";
import type { ProcessStatus } from "../../types";
import { useProcessesStore } from "../../stores/processes";
import { useConfigStore } from "../../stores/config";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { PlayIcon, StopIcon, ReloadIcon, EyeOpenIcon } from "@radix-icons/vue";

const props = defineProps<{
  projectId: string;
  groupId: string;
  status: ProcessStatus;
  autoRestart: boolean;
  watchPatterns?: string[];
  getTerminalDimensions?: () => { cols: number; rows: number } | null;
}>();

const emit = defineEmits<{
  editWatchPatterns: [];
}>();

const processes = useProcessesStore();
const config = useConfigStore();
const loading = ref(false);

const isRunning = computed(() => props.status === "running");

async function start() {
  loading.value = true;
  try {
    const dims = props.getTerminalDimensions?.();
    await processes.startProcess(props.groupId, props.projectId, dims?.cols, dims?.rows);
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
    const dims = props.getTerminalDimensions?.();
    await processes.restartProcess(props.groupId, props.projectId, dims?.cols, dims?.rows);
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
    <Button
      v-if="!isRunning"
      size="sm"
      :disabled="loading"
      @click="start"
      class="gap-1.5"
    >
      <PlayIcon class="h-3.5 w-3.5" />
      Start
    </Button>
    <Button
      v-if="isRunning"
      variant="destructive"
      size="sm"
      :disabled="loading"
      @click="stop"
      class="gap-1.5"
    >
      <StopIcon class="h-3.5 w-3.5" />
      Stop
    </Button>
    <Button
      variant="secondary"
      size="sm"
      :disabled="loading"
      @click="restart"
      class="gap-1.5"
    >
      <ReloadIcon class="h-3.5 w-3.5" />
      Restart
    </Button>

    <div class="ml-auto flex items-center gap-2">
      <Label for="auto-restart" class="text-xs text-muted-foreground cursor-pointer">Auto-restart</Label>
      <Switch
        id="auto-restart"
        :model-value="autoRestart"
        @update:model-value="toggleAutoRestart"
      />
      <Button
        v-if="autoRestart"
        variant="ghost"
        size="icon"
        class="h-7 w-7"
        title="Edit Watch Patterns"
        @click="emit('editWatchPatterns')"
      >
        <EyeOpenIcon class="h-3.5 w-3.5" />
      </Button>
    </div>
  </div>
</template>
