<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useSessionsStore } from "../../stores/sessions";
import { useUiStore } from "../../stores/ui";
import type { SessionWithStats } from "../../types";
import ConfirmDialog from "../shared/ConfirmDialog.vue";

const props = defineProps<{
  projectId: string;
  projectName: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

const sessions = useSessionsStore();
const ui = useUiStore();
const deleteSessionId = ref<string | null>(null);

onMounted(() => {
  sessions.loadSessionsWithStats(props.projectId);
});

function formatDate(ts: number): string {
  return new Date(ts).toLocaleString();
}

function formatDuration(session: SessionWithStats): string {
  if (!session.endedAt) return "Running";
  const ms = session.endedAt - session.startedAt;
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const secs = seconds % 60;
  if (minutes < 60) return `${minutes}m ${secs}s`;
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  return `${hours}h ${mins}m`;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function statusColor(session: SessionWithStats): string {
  if (!session.endedAt) return "text-green-400";
  if (session.exitStatus === "errored") return "text-red-400";
  return "text-gray-400";
}

function statusLabel(session: SessionWithStats): string {
  if (!session.endedAt) return "Running";
  if (session.exitStatus === "errored") return "Errored";
  return "Stopped";
}

function viewSession(sessionId: string) {
  ui.showSessionDetail(sessionId);
}

async function handleDeleteSession() {
  if (deleteSessionId.value) {
    await sessions.deleteSession(deleteSessionId.value);
    deleteSessionId.value = null;
    sessions.loadSessionsWithStats(props.projectId);
  }
}
</script>

<template>
  <div class="flex-1 flex flex-col h-full min-h-0">
    <div class="p-4 border-b border-gray-700 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <button
          class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
          @click="emit('close')"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <h2 class="text-lg font-semibold text-gray-100">
          Sessions - {{ props.projectName }}
        </h2>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto p-4">
      <div v-if="sessions.loading" class="text-center text-gray-500 py-8">
        Loading sessions...
      </div>
      <div v-else-if="sessions.sessionsWithStats.length === 0" class="text-center text-gray-500 py-8">
        No sessions recorded yet. Start the process to create a session.
      </div>
      <div v-else class="space-y-2">
        <div
          v-for="session in sessions.sessionsWithStats"
          :key="session.id"
          class="bg-gray-800 rounded-lg p-3 border border-gray-700 hover:border-gray-600 transition-colors cursor-pointer"
          @click="viewSession(session.id)"
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <span class="text-sm text-gray-300">{{ formatDate(session.startedAt) }}</span>
              <span :class="statusColor(session)" class="text-xs font-medium">
                {{ statusLabel(session) }}
              </span>
            </div>
            <div class="flex items-center gap-3">
              <span class="text-xs text-gray-500">{{ formatDuration(session) }}</span>
              <button
                class="p-1 text-gray-500 hover:text-red-400 transition-colors"
                title="Delete session"
                @click.stop="deleteSessionId = session.id"
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>
          </div>
          <div class="flex items-center gap-3 mt-1.5 text-[11px] text-gray-500">
            <span>{{ session.logCount }} logs</span>
            <span>{{ formatSize(session.logSize) }}</span>
            <span>{{ session.metricCount }} metrics</span>
          </div>
        </div>
      </div>
    </div>

    <ConfirmDialog
      :open="!!deleteSessionId"
      title="Delete Session"
      message="Are you sure you want to delete this session and all its logs?"
      @confirm="handleDeleteSession"
      @cancel="deleteSessionId = null"
    />
  </div>
</template>
