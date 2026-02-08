<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { Group, Project, Session, MetricPoint } from "../../types";
import { invoke } from "@tauri-apps/api/core";
import { useProcessesStore } from "../../stores/processes";
import { useConfigStore } from "../../stores/config";
import { useUiStore } from "../../stores/ui";
import { useSessionsStore } from "../../stores/sessions";
import StatusBadge from "../shared/StatusBadge.vue";
import ProcessControls from "./ProcessControls.vue";
import ProcessStats from "./ProcessStats.vue";
import LogPanel from "./LogPanel.vue";
import MonitorGraph from "./MonitorGraph.vue";
import SessionsList from "./SessionsList.vue";
import ProjectFormDialog from "../shared/ProjectFormDialog.vue";
import ConfirmDialog from "../shared/ConfirmDialog.vue";

const props = defineProps<{
  project: Project;
  group: Group;
}>();

const processes = useProcessesStore();
const config = useConfigStore();
const ui = useUiStore();
const sessionsStore = useSessionsStore();

const showEditDialog = ref(false);
const showDeleteDialog = ref(false);
const showSessions = ref(false);

const processInfo = computed(() => processes.getStatus(props.project.id));
const status = computed(() => processInfo.value?.status ?? "stopped");
const cpuUsage = computed(() => processInfo.value?.cpuUsage ?? 0);
const memoryUsage = computed(() => processInfo.value?.memoryUsage ?? 0);

const groupRunningCount = computed(() =>
  props.group.projects.filter((p) => processes.getStatus(p.id)?.status === "running").length,
);
const groupTotalCount = computed(() => props.group.projects.length);
const allRunning = computed(() => groupRunningCount.value === groupTotalCount.value && groupTotalCount.value > 0);
const someRunning = computed(() => groupRunningCount.value > 0);

const startStopAllLoading = ref(false);
async function startAllInGroup() {
  startStopAllLoading.value = true;
  try {
    await processes.startAllInGroup(
      props.group.id,
      props.group.projects.map((p) => p.id),
    );
  } finally {
    startStopAllLoading.value = false;
  }
}

async function stopAllInGroup() {
  startStopAllLoading.value = true;
  try {
    await processes.stopAllInGroup(props.group.projects.map((p) => p.id));
  } finally {
    startStopAllLoading.value = false;
  }
}

const lastSession = ref<Session | null>(null);
const lastMetric = ref<MetricPoint | null>(null);
const lastSessionMetrics = ref<MetricPoint[]>([]);

async function loadLastSessionData() {
  if (status.value === "running") {
    lastSession.value = null;
    lastMetric.value = null;
    lastSessionMetrics.value = [];
    return;
  }
  try {
    const session = await sessionsStore.getLastSession(props.project.id);
    lastSession.value = session;
    if (session) {
      lastMetric.value = await sessionsStore.getLastMetric(session.id);
      lastSessionMetrics.value = await sessionsStore.getSessionMetrics(session.id);
    } else {
      lastMetric.value = null;
      lastSessionMetrics.value = [];
    }
  } catch {
    lastSession.value = null;
    lastMetric.value = null;
    lastSessionMetrics.value = [];
  }
}

function formatDate(ts: number): string {
  return new Date(ts).toLocaleString();
}

function formatDuration(start: number, end: number): string {
  const diff = end - start;
  const seconds = Math.floor(diff / 1000) % 60;
  const minutes = Math.floor(diff / 60000) % 60;
  const hours = Math.floor(diff / 3600000);
  if (hours > 0) return `${hours}h ${minutes}m ${seconds}s`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
}

function formatMemory(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

watch(
  [() => status.value, () => props.project.id],
  () => loadLastSessionData(),
  { immediate: true },
);

async function handleEdit(
  name: string,
  command: string,
  cwd?: string,
  envVars?: Record<string, string>,
) {
  await config.updateProject(props.group.id, props.project.id, {
    name,
    command,
    cwd: cwd || null,
    envVars: envVars ?? {},
  });
  showEditDialog.value = false;
}

async function handleDelete() {
  if (status.value === "running") {
    await processes.stopProcess(props.project.id);
  }
  await config.deleteProject(props.group.id, props.project.id);
  showDeleteDialog.value = false;
  ui.clearSelection();
}
</script>

<template>
  <div class="flex-1 flex flex-col h-full min-h-0">
    <!-- Group Navbar -->
    <div class="px-4 py-2 border-b border-gray-700/50 bg-gray-800/50 flex items-center justify-between">
      <div class="flex items-center gap-2 min-w-0">
        <button
          class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
          title="Back"
          @click="ui.clearSelection()"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <span class="text-xs text-gray-400 truncate">{{ props.group.name }}</span>
        <span class="text-xs text-gray-500">/</span>
        <span class="text-xs text-gray-300 font-medium truncate">{{ props.project.name }}</span>
        <span class="text-[10px] px-1.5 py-0.5 rounded-full ml-1"
          :class="someRunning ? 'bg-green-900/30 text-green-400' : 'bg-gray-700 text-gray-500'"
        >
          {{ groupRunningCount }}/{{ groupTotalCount }}
        </span>
      </div>
      <div class="flex items-center gap-1.5">
        <button
          class="px-2 py-1 text-[11px] rounded transition-colors flex items-center gap-1 disabled:opacity-50"
          :class="allRunning
            ? 'bg-red-600/20 text-red-400 hover:bg-red-600/30'
            : 'bg-green-600/20 text-green-400 hover:bg-green-600/30'"
          :disabled="startStopAllLoading"
          :title="allRunning ? 'Stop All' : 'Start All'"
          @click="allRunning ? stopAllInGroup() : startAllInGroup()"
        >
          <svg v-if="allRunning" class="w-3 h-3" fill="currentColor" viewBox="0 0 24 24">
            <rect x="6" y="6" width="12" height="12" rx="1" />
          </svg>
          <svg v-else class="w-3 h-3" fill="currentColor" viewBox="0 0 24 24">
            <path d="M8 5v14l11-7z" />
          </svg>
          <span>{{ allRunning ? 'Stop All' : 'Start All' }}</span>
        </button>
        <button
          class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
          title="Group Monitor"
          @click="ui.showGroupMonitor(props.group.id)"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Header -->
    <div class="p-4 border-b border-gray-700 space-y-3">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h2 class="text-lg font-semibold text-gray-100">
            {{ props.project.name }}
          </h2>
          <StatusBadge :status="status" />
        </div>
        <div class="flex items-center gap-1">
          <button
            class="p-1.5 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
            :class="ui.showMonitor ? 'bg-gray-700 text-gray-200' : ''"
            title="Monitor"
            @click="ui.showMonitor = !ui.showMonitor"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
          </button>
          <button
            class="p-1.5 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
            title="Sessions"
            @click="showSessions = true"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </button>
          <button
            class="p-1.5 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
            title="Edit Project"
            @click="showEditDialog = true"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
              />
            </svg>
          </button>
          <button
            class="p-1.5 rounded hover:bg-gray-700 text-gray-400 hover:text-red-400 transition-colors"
            title="Delete Project"
            @click="showDeleteDialog = true"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
              />
            </svg>
          </button>
        </div>
      </div>

      <div class="text-xs text-gray-500 font-mono bg-gray-900 px-2 py-1 rounded">
        <span>{{ props.project.command }}</span>
        <span v-if="props.project.cwd" class="text-gray-600 ml-2">cwd: {{ props.project.cwd }}</span>
      </div>

      <ProcessControls
        :project-id="props.project.id"
        :group-id="props.group.id"
        :status="status"
        :auto-restart="props.project.autoRestart"
      />

      <ProcessStats
        v-if="status === 'running'"
        :cpu-usage="cpuUsage"
        :memory-usage="memoryUsage"
      />

      <!-- Last session info for stopped/errored projects -->
      <div v-else-if="lastSession" class="flex flex-wrap items-center gap-3 text-xs text-gray-400">
        <div class="flex items-center gap-1">
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span>Last run: {{ formatDate(lastSession.startedAt) }}</span>
        </div>
        <div v-if="lastSession.endedAt" class="flex items-center gap-1">
          <span>Duration: {{ formatDuration(lastSession.startedAt, lastSession.endedAt) }}</span>
        </div>
        <div v-if="lastSession.exitStatus" class="flex items-center gap-1">
          <span :class="lastSession.exitStatus === 'errored' ? 'text-red-400' : 'text-gray-400'">
            Exit: {{ lastSession.exitStatus }}
          </span>
        </div>
        <div v-if="lastMetric" class="flex items-center gap-2">
          <span>CPU: {{ lastMetric.cpuUsage.toFixed(1) }}%</span>
          <span>MEM: {{ formatMemory(lastMetric.memoryUsage) }}</span>
        </div>
      </div>
    </div>

    <!-- Monitor Graph -->
    <MonitorGraph
      v-if="ui.showMonitor"
      :project-id="props.project.id"
      :initial-data="status !== 'running' ? lastSessionMetrics : undefined"
    />

    <!-- Sessions List or Log Panel -->
    <SessionsList
      v-if="showSessions"
      :project-id="props.project.id"
      :project-name="props.project.name"
      @close="showSessions = false"
    />
    <LogPanel v-else :project-id="props.project.id" :group-id="props.group.id" />

    <!-- Dialogs -->
    <ProjectFormDialog
      :open="showEditDialog"
      title="Edit Project"
      :name="props.project.name"
      :command="props.project.command"
      :cwd="props.project.cwd ?? undefined"
      :env-vars="props.project.envVars"
      @confirm="handleEdit"
      @cancel="showEditDialog = false"
    />

    <ConfirmDialog
      :open="showDeleteDialog"
      title="Delete Project"
      :message="`Are you sure you want to delete '${props.project.name}'?`"
      @confirm="handleDelete"
      @cancel="showDeleteDialog = false"
    />
  </div>
</template>
