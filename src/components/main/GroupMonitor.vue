<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useConfigStore } from "../../stores/config";
import { useProcessesStore } from "../../stores/processes";
import { useSessionsStore } from "../../stores/sessions";
import { useUiStore } from "../../stores/ui";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ProcessInfo, Session, MetricPoint, ProjectType } from "../../types";
import StatusBadge from "../shared/StatusBadge.vue";
import ConfirmDialog from "../shared/ConfirmDialog.vue";

const props = defineProps<{
  groupId: string;
}>();

const config = useConfigStore();
const processes = useProcessesStore();
const sessionsStore = useSessionsStore();
const ui = useUiStore();

const group = computed(() =>
  config.groups.find((g) => g.id === props.groupId),
);

const selectedType = ref<ProjectType>("service");

const filteredProjects = computed(() => {
  if (!group.value) return [];
  return group.value.projects.filter((p) => p.projectType === selectedType.value);
});

const runningProjects = computed(() =>
  filteredProjects.value.filter((p) => getStatus(p.id) === "running"),
);

const stoppedProjects = computed(() =>
  filteredProjects.value.filter((p) => getStatus(p.id) !== "running"),
);

// Track mini sparkline data per project
const sparklineData = ref<Map<string, { cpu: number[]; mem: number[] }>>(
  new Map(),
);

// Track last session and metrics per project (for stopped projects)
const lastSessions = ref<Map<string, Session>>(new Map());
const lastMetrics = ref<Map<string, MetricPoint>>(new Map());
const recentLogs = ref<Map<string, string>>(new Map());
const now = ref(Date.now());

const MAX_SPARKLINE = 30;

let unlisten: UnlistenFn | null = null;
let uptimeInterval: ReturnType<typeof setInterval> | null = null;

const groupRunningCount = computed(() =>
  filteredProjects.value.filter((p) => getStatus(p.id) === "running").length,
);
const groupTotalCount = computed(() => filteredProjects.value.length);
const allRunning = computed(() => groupRunningCount.value === groupTotalCount.value && groupTotalCount.value > 0);

const startStopAllLoading = ref(false);

// Context menu state
const showContextMenu = ref(false);
const contextMenuPos = ref({ x: 0, y: 0 });
const contextMenuProject = ref<{ id: string; name: string; projectType: ProjectType } | null>(null);
const showDeleteDialog = ref(false);
async function startAll() {
  if (!group.value) return;
  startStopAllLoading.value = true;
  try {
    // Only start services, not tasks
    const serviceProjects = filteredProjects.value.filter((p) => p.projectType === "service");
    await processes.startAllInGroup(
      group.value.id,
      serviceProjects.map((p) => p.id),
    );
  } finally {
    startStopAllLoading.value = false;
  }
}

async function stopAll() {
  if (!group.value) return;
  startStopAllLoading.value = true;
  try {
    await processes.stopAllInGroup(group.value.projects.map((p) => p.id));
  } finally {
    startStopAllLoading.value = false;
  }
}

async function toggleProject(projectId: string) {
  if (!group.value) return;
  const status = getStatus(projectId);
  if (status === "running") {
    await processes.stopProcess(projectId);
  } else {
    await processes.startProcess(group.value.id, projectId);
  }
}

function onProjectContextMenu(e: MouseEvent, project: { id: string; name: string; projectType: ProjectType }) {
  e.preventDefault();
  e.stopPropagation();
  contextMenuPos.value = { x: e.clientX, y: e.clientY };
  contextMenuProject.value = project;
  showContextMenu.value = true;
  document.addEventListener("click", () => {
    showContextMenu.value = false;
  }, { once: true });
}

async function handleConvertProject(newType: ProjectType) {
  if (!group.value || !contextMenuProject.value) return;
  await config.convertMultipleProjects(group.value.id, [contextMenuProject.value.id], newType);
  showContextMenu.value = false;
}

async function handleDeleteProject() {
  if (!group.value || !contextMenuProject.value) return;
  await config.deleteProject(group.value.id, contextMenuProject.value.id);
  showContextMenu.value = false;
  showDeleteDialog.value = false;
}

function getStatus(projectId: string) {
  return processes.getStatus(projectId)?.status ?? "stopped";
}

function getCpu(projectId: string) {
  return processes.getStatus(projectId)?.cpuUsage ?? 0;
}

function getPid(projectId: string) {
  return processes.getStatus(projectId)?.pid ?? null;
}

function formatMemory(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function getMemory(projectId: string) {
  const bytes = processes.getStatus(projectId)?.memoryUsage ?? 0;
  return formatMemory(bytes);
}

function getSparklinePath(
  data: number[],
  width: number,
  height: number,
): string {
  if (data.length < 2) return "";
  const max = Math.max(...data, 1);
  const step = width / (data.length - 1);
  return data
    .map((v, i) => {
      const x = i * step;
      const y = height - (v / max) * height;
      return `${i === 0 ? "M" : "L"}${x},${y}`;
    })
    .join(" ");
}

function getProjectSparkline(projectId: string) {
  return sparklineData.value.get(projectId) ?? { cpu: [], mem: [] };
}

function formatUptime(startedAt: number): string {
  const diff = now.value - startedAt;
  const seconds = Math.floor(diff / 1000) % 60;
  const minutes = Math.floor(diff / 60000) % 60;
  const hours = Math.floor(diff / 3600000);
  if (hours > 0) return `${hours}h ${minutes}m ${seconds}s`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
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

function getLogLines(projectId: string): string[] {
  const raw = recentLogs.value.get(projectId) ?? "";
  if (!raw) return [];
  // Strip ANSI escape codes for plain text display
  const stripped = raw.replace(/\x1b\[[0-9;]*m/g, "");
  const lines = stripped.split("\n").filter((l) => l.trim().length > 0);
  return lines.slice(-5);
}

async function loadProjectData() {
  const projects = group.value?.projects ?? [];
  for (const project of projects) {
    try {
      // Load recent logs for all projects
      const logs = await sessionsStore.getRecentLogs(project.id, 10);
      if (logs) recentLogs.value.set(project.id, logs);

      // Load last session for non-running projects
      if (getStatus(project.id) !== "running") {
        const session = await sessionsStore.getLastSession(project.id);
        if (session) {
          lastSessions.value.set(project.id, session);
          const metric = await sessionsStore.getLastMetric(session.id);
          if (metric) lastMetrics.value.set(project.id, metric);
        }
      }
    } catch {
      // Ignore errors for individual projects
    }
  }
}

onMounted(async () => {
  unlisten = await listen<ProcessInfo[]>("process-stats-updated", (event) => {
    for (const info of event.payload) {
      if (!group.value?.projects.some((p) => p.id === info.projectId))
        continue;

      let data = sparklineData.value.get(info.projectId);
      if (!data) {
        data = { cpu: [], mem: [] };
        sparklineData.value.set(info.projectId, data);
      }
      data.cpu.push(info.cpuUsage);
      data.mem.push(info.memoryUsage / (1024 * 1024));
      if (data.cpu.length > MAX_SPARKLINE) {
        data.cpu.shift();
        data.mem.shift();
      }
    }
  });

  uptimeInterval = setInterval(() => {
    now.value = Date.now();
  }, 1000);

  await loadProjectData();
});

onUnmounted(() => {
  unlisten?.();
  if (uptimeInterval) clearInterval(uptimeInterval);
});
</script>

<template>
  <div class="flex-1 flex flex-col h-full min-h-0">
    <div class="p-4 border-b border-gray-700 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <button
          class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
          @click="ui.clearSelection()"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <h2 class="text-lg font-semibold text-gray-100">
          {{ group?.name ?? "Unknown" }}
        </h2>
        <span class="text-xs px-1.5 py-0.5 rounded-full"
          :class="groupRunningCount > 0 ? 'bg-green-900/30 text-green-400' : 'bg-gray-700 text-gray-500'"
        >
          {{ groupRunningCount }}/{{ groupTotalCount }} running
        </span>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="px-3 py-1.5 text-xs rounded transition-colors flex items-center gap-1.5 disabled:opacity-50"
          :class="allRunning
            ? 'bg-red-600 text-white hover:bg-red-500'
            : 'bg-green-600 text-white hover:bg-green-500'"
          :disabled="startStopAllLoading"
          @click="allRunning ? stopAll() : startAll()"
        >
          <svg v-if="allRunning" class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
            <rect x="6" y="6" width="12" height="12" rx="1" />
          </svg>
          <svg v-else class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M8 5v14l11-7z" />
          </svg>
          {{ allRunning ? 'Stop All' : 'Start All' }}
        </button>
      </div>
    </div>

    <!-- Type tabs -->
    <div class="px-4 pt-3 border-b border-gray-700">
      <div class="flex gap-1">
        <button
          class="px-4 py-1.5 text-sm rounded-t transition-colors"
          :class="selectedType === 'service'
            ? 'bg-gray-700 text-gray-100 border-t border-l border-r border-gray-600'
            : 'text-gray-500 hover:text-gray-300 hover:bg-gray-800'"
          @click="selectedType = 'service'"
        >
          Services
        </button>
        <button
          class="px-4 py-1.5 text-sm rounded-t transition-colors"
          :class="selectedType === 'task'
            ? 'bg-gray-700 text-gray-100 border-t border-l border-r border-gray-600'
            : 'text-gray-500 hover:text-gray-300 hover:bg-gray-800'"
          @click="selectedType = 'task'"
        >
          Tasks
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto p-4">
      <div v-if="!group || filteredProjects.length === 0" class="text-center text-gray-500 py-8">
        {{ !group ? 'No group selected.' : `No ${selectedType}s in this group.` }}
      </div>
      <div v-else class="space-y-6">
        <!-- Running Section -->
        <div v-if="runningProjects.length > 0">
          <h3 class="text-xs font-semibold text-green-400 uppercase tracking-wider mb-3 flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-green-400 animate-pulse"></span>
            Running ({{ runningProjects.length }})
          </h3>
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
            <div
              v-for="project in runningProjects"
              :key="project.id"
              class="bg-gray-800 rounded-lg p-4 border border-gray-700 hover:border-gray-600 transition-colors cursor-pointer"
              @click="ui.selectProject(props.groupId, project.id)"
              @contextmenu.prevent="onProjectContextMenu($event, project)"
            >
          <!-- Header: name + badges + play/stop -->
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-2 min-w-0">
              <span class="text-sm font-medium text-gray-200 truncate">{{
                project.name
              }}</span>
              <span
                v-if="project.autoRestart"
                class="text-[10px] px-1 py-0.5 rounded bg-blue-900/30 text-blue-400 flex-shrink-0"
              >auto</span>
            </div>
            <div class="flex items-center gap-2 flex-shrink-0">
              <button
                class="p-1 rounded transition-colors"
                :class="getStatus(project.id) === 'running'
                  ? 'hover:bg-red-900/30 text-red-400 hover:text-red-300'
                  : 'hover:bg-green-900/30 text-green-400 hover:text-green-300'"
                :title="getStatus(project.id) === 'running' ? 'Stop' : 'Start'"
                @click.stop="toggleProject(project.id)"
              >
                <svg v-if="getStatus(project.id) === 'running'" class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                  <rect x="6" y="6" width="12" height="12" rx="1" />
                </svg>
                <svg v-else class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M8 5v14l11-7z" />
                </svg>
              </button>
              <StatusBadge :status="getStatus(project.id)" />
            </div>
          </div>

          <!-- Running project details -->
          <template v-if="getStatus(project.id) === 'running'">
            <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-gray-400 mb-2">
              <span>CPU: {{ getCpu(project.id).toFixed(1) }}%</span>
              <span>MEM: {{ getMemory(project.id) }}</span>
              <span v-if="getPid(project.id)">PID: {{ getPid(project.id) }}</span>
            </div>

            <div class="grid grid-cols-2 gap-2 mb-3">
              <div>
                <span class="text-[10px] text-gray-500">CPU</span>
                <svg
                  class="w-full h-6"
                  viewBox="0 0 100 20"
                  preserveAspectRatio="none"
                >
                  <path
                    :d="
                      getSparklinePath(
                        getProjectSparkline(project.id).cpu,
                        100,
                        20,
                      )
                    "
                    fill="none"
                    stroke="#3b82f6"
                    stroke-width="1.5"
                  />
                </svg>
              </div>
              <div>
                <span class="text-[10px] text-gray-500">Memory</span>
                <svg
                  class="w-full h-6"
                  viewBox="0 0 100 20"
                  preserveAspectRatio="none"
                >
                  <path
                    :d="
                      getSparklinePath(
                        getProjectSparkline(project.id).mem,
                        100,
                        20,
                      )
                    "
                    fill="none"
                    stroke="#10b981"
                    stroke-width="1.5"
                  />
                </svg>
              </div>
            </div>

            <!-- Recent output for running -->
            <div v-if="getLogLines(project.id).length > 0">
              <span class="text-[10px] text-gray-500">Recent Output</span>
              <div class="mt-1 bg-gray-900 rounded p-2 max-h-24 overflow-hidden">
                <div
                  v-for="(line, i) in getLogLines(project.id)"
                  :key="i"
                  class="text-[11px] text-gray-400 font-mono truncate leading-tight"
                >{{ line }}</div>
              </div>
            </div>
          </template>

          <!-- Stopped/errored project details -->
          <template v-else>
            <div v-if="lastSessions.get(project.id)">
              <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-gray-500 mb-2">
                <span>{{ formatDate(lastSessions.get(project.id)!.startedAt) }}</span>
                <span v-if="lastSessions.get(project.id)!.endedAt">
                  Duration: {{ formatDuration(lastSessions.get(project.id)!.startedAt, lastSessions.get(project.id)!.endedAt!) }}
                </span>
                <span
                  v-if="lastSessions.get(project.id)!.exitStatus"
                  :class="lastSessions.get(project.id)!.exitStatus === 'errored' ? 'text-red-400' : 'text-gray-500'"
                >
                  Exit: {{ lastSessions.get(project.id)!.exitStatus }}
                </span>
              </div>
              <div v-if="lastMetrics.get(project.id)" class="flex items-center gap-3 text-xs text-gray-500 mb-2">
                <span>Last CPU: {{ lastMetrics.get(project.id)!.cpuUsage.toFixed(1) }}%</span>
                <span>Last MEM: {{ formatMemory(lastMetrics.get(project.id)!.memoryUsage) }}</span>
              </div>

              <!-- Last output for stopped -->
              <div v-if="getLogLines(project.id).length > 0">
                <span class="text-[10px] text-gray-500">Last Output</span>
                <div class="mt-1 bg-gray-900 rounded p-2 max-h-24 overflow-hidden">
                  <div
                    v-for="(line, i) in getLogLines(project.id)"
                    :key="i"
                    class="text-[11px] text-gray-500 font-mono truncate leading-tight"
                  >{{ line }}</div>
                </div>
              </div>
            </div>
            <div v-else class="text-xs text-gray-600">No session history</div>
          </template>
        </div>
          </div>
        </div>

        <!-- Stopped Section -->
        <div v-if="stoppedProjects.length > 0">
          <h3 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-3 flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-gray-500"></span>
            Stopped ({{ stoppedProjects.length }})
          </h3>
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
            <div
              v-for="project in stoppedProjects"
              :key="project.id"
              class="bg-gray-800 rounded-lg p-4 border border-gray-700 hover:border-gray-600 transition-colors cursor-pointer"
              @click="ui.selectProject(props.groupId, project.id)"
              @contextmenu.prevent="onProjectContextMenu($event, project)"
            >
          <!-- Header: name + badges + play/stop -->
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-2 min-w-0">
              <span class="text-sm font-medium text-gray-200 truncate">{{
                project.name
              }}</span>
              <span
                v-if="project.autoRestart"
                class="text-[10px] px-1 py-0.5 rounded bg-blue-900/30 text-blue-400 flex-shrink-0"
              >auto</span>
            </div>
            <div class="flex items-center gap-2 flex-shrink-0">
              <button
                class="p-1 rounded transition-colors"
                :class="getStatus(project.id) === 'running'
                  ? 'hover:bg-red-900/30 text-red-400 hover:text-red-300'
                  : 'hover:bg-green-900/30 text-green-400 hover:text-green-300'"
                :title="getStatus(project.id) === 'running' ? 'Stop' : 'Start'"
                @click.stop="toggleProject(project.id)"
              >
                <svg v-if="getStatus(project.id) === 'running'" class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                  <rect x="6" y="6" width="12" height="12" rx="1" />
                </svg>
                <svg v-else class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M8 5v14l11-7z" />
                </svg>
              </button>
              <StatusBadge :status="getStatus(project.id)" />
            </div>
          </div>

          <!-- Running project details -->
          <template v-if="getStatus(project.id) === 'running'">
            <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-gray-400 mb-2">
              <span>CPU: {{ getCpu(project.id).toFixed(1) }}%</span>
              <span>MEM: {{ getMemory(project.id) }}</span>
              <span v-if="getPid(project.id)">PID: {{ getPid(project.id) }}</span>
            </div>

            <div class="grid grid-cols-2 gap-2 mb-3">
              <div>
                <span class="text-[10px] text-gray-500">CPU</span>
                <svg
                  class="w-full h-6"
                  viewBox="0 0 100 20"
                  preserveAspectRatio="none"
                >
                  <path
                    :d="
                      getSparklinePath(
                        getProjectSparkline(project.id).cpu,
                        100,
                        20,
                      )
                    "
                    fill="none"
                    stroke="#3b82f6"
                    stroke-width="1.5"
                  />
                </svg>
              </div>
              <div>
                <span class="text-[10px] text-gray-500">Memory</span>
                <svg
                  class="w-full h-6"
                  viewBox="0 0 100 20"
                  preserveAspectRatio="none"
                >
                  <path
                    :d="
                      getSparklinePath(
                        getProjectSparkline(project.id).mem,
                        100,
                        20,
                      )
                    "
                    fill="none"
                    stroke="#10b981"
                    stroke-width="1.5"
                  />
                </svg>
              </div>
            </div>

            <!-- Recent output for running -->
            <div v-if="getLogLines(project.id).length > 0">
              <span class="text-[10px] text-gray-500">Recent Output</span>
              <div class="mt-1 bg-gray-900 rounded p-2 max-h-24 overflow-hidden">
                <div
                  v-for="(line, i) in getLogLines(project.id)"
                  :key="i"
                  class="text-[11px] text-gray-400 font-mono truncate leading-tight"
                >{{ line }}</div>
              </div>
            </div>
          </template>

          <!-- Stopped/errored project details -->
          <template v-else>
            <div v-if="lastSessions.get(project.id)">
              <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-gray-500 mb-2">
                <span>{{ formatDate(lastSessions.get(project.id)!.startedAt) }}</span>
                <span v-if="lastSessions.get(project.id)!.endedAt">
                  Duration: {{ formatDuration(lastSessions.get(project.id)!.startedAt, lastSessions.get(project.id)!.endedAt!) }}
                </span>
                <span
                  v-if="lastSessions.get(project.id)!.exitStatus"
                  :class="lastSessions.get(project.id)!.exitStatus === 'errored' ? 'text-red-400' : 'text-gray-500'"
                >
                  Exit: {{ lastSessions.get(project.id)!.exitStatus }}
                </span>
              </div>
              <div v-if="lastMetrics.get(project.id)" class="flex items-center gap-3 text-xs text-gray-500 mb-2">
                <span>Last CPU: {{ lastMetrics.get(project.id)!.cpuUsage.toFixed(1) }}%</span>
                <span>Last MEM: {{ formatMemory(lastMetrics.get(project.id)!.memoryUsage) }}</span>
              </div>

              <!-- Last output for stopped -->
              <div v-if="getLogLines(project.id).length > 0">
                <span class="text-[10px] text-gray-500">Last Output</span>
                <div class="mt-1 bg-gray-900 rounded p-2 max-h-24 overflow-hidden">
                  <div
                    v-for="(line, i) in getLogLines(project.id)"
                    :key="i"
                    class="text-[11px] text-gray-500 font-mono truncate leading-tight"
                  >{{ line }}</div>
                </div>
              </div>
            </div>
            <div v-else class="text-xs text-gray-600">No session history</div>
          </template>
        </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Project Context Menu -->
    <Teleport to="body">
      <div
        v-if="showContextMenu && contextMenuProject"
        class="fixed z-50 min-w-[12rem] bg-gray-800 border border-gray-700 rounded-lg shadow-xl py-1"
        :style="{ left: contextMenuPos.x + 'px', top: contextMenuPos.y + 'px' }"
      >
        <div class="px-3 py-1.5 text-xs text-gray-500 border-b border-gray-700">
          {{ contextMenuProject.name }}
        </div>
        <button
          v-if="contextMenuProject.projectType !== 'service'"
          class="w-full px-3 py-1.5 text-left text-sm text-gray-300 hover:bg-gray-700 transition-colors"
          @click="handleConvertProject('service')"
        >
          Convert to Service
        </button>
        <button
          v-if="contextMenuProject.projectType !== 'task'"
          class="w-full px-3 py-1.5 text-left text-sm text-gray-300 hover:bg-gray-700 transition-colors"
          @click="handleConvertProject('task')"
        >
          Convert to Task
        </button>
        <div class="border-t border-gray-700 my-1"></div>
        <button
          class="w-full px-3 py-1.5 text-left text-sm text-red-400 hover:bg-gray-700 transition-colors"
          @click="showDeleteDialog = true"
        >
          Delete Project
        </button>
      </div>
    </Teleport>

    <!-- Delete Confirmation Dialog -->
    <ConfirmDialog
      :open="showDeleteDialog"
      title="Delete Project"
      :message="`Delete '${contextMenuProject?.name}'? This action cannot be undone.`"
      @confirm="handleDeleteProject"
      @cancel="showDeleteDialog = false"
    />
  </div>
</template>
