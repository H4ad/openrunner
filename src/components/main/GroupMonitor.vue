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
import SparklineChart from "../shared/SparklineChart.vue";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import { PlayIcon, StopIcon, ArrowLeftIcon, ActivityLogIcon, DesktopIcon, LayersIcon, FileTextIcon, ClockIcon } from "@radix-icons/vue";

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
  const grp = group.value;
  if (!grp) return;
  for (const project of projects) {
    try {
      // Load recent logs for all projects
      const logs = await sessionsStore.getRecentLogs(grp.id, project.id, 10);
      if (logs) recentLogs.value.set(project.id, logs);

      // Load last session for non-running projects
      if (getStatus(project.id) !== "running") {
        const session = await sessionsStore.getLastSession(grp.id, project.id);
        if (session) {
          lastSessions.value.set(project.id, session);
          const metric = await sessionsStore.getLastMetric(grp.id, session.id);
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
      // Create new arrays to trigger Vue reactivity
      const newCpu = [...data.cpu, info.cpuUsage];
      const newMem = [...data.mem, info.memoryUsage / (1024 * 1024)];
      if (newCpu.length > MAX_SPARKLINE) {
        newCpu.shift();
        newMem.shift();
      }
      sparklineData.value.set(info.projectId, { cpu: newCpu, mem: newMem });
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
    <div class="p-4 border-b border-border flex items-center justify-between">
      <div class="flex items-center gap-3">
        <Button variant="ghost" size="icon" class="h-8 w-8" @click="ui.clearSelection()">
          <ArrowLeftIcon class="h-4 w-4" />
        </Button>
        <h2 class="text-lg font-semibold text-foreground">
          {{ group?.name ?? "Unknown" }}
        </h2>
        <Badge
          :variant="groupRunningCount > 0 ? 'default' : 'secondary'"
          class="text-xs"
        >
          {{ groupRunningCount }}/{{ groupTotalCount }} running
        </Badge>
      </div>
      <Button
        :variant="allRunning ? 'destructive' : 'default'"
        size="sm"
        :disabled="startStopAllLoading"
        @click="allRunning ? stopAll() : startAll()"
        class="gap-1.5"
      >
        <StopIcon v-if="allRunning" class="h-3.5 w-3.5" />
        <PlayIcon v-else class="h-3.5 w-3.5" />
        {{ allRunning ? 'Stop All' : 'Start All' }}
      </Button>
    </div>

    <!-- Type tabs -->
    <Tabs v-model="selectedType" class="w-full">
      <div class="px-4 pt-3 border-b border-border">
        <TabsList>
          <TabsTrigger value="service">Services</TabsTrigger>
          <TabsTrigger value="task">Tasks</TabsTrigger>
        </TabsList>
      </div>

      <TabsContent value="service" class="m-0">
        <div class="flex-1 overflow-y-auto p-4">
          <div v-if="!group || runningProjects.length === 0 && stoppedProjects.length === 0" class="text-center text-muted-foreground py-8">
            No services in this group.
          </div>
          <div v-else class="space-y-6">
            <!-- Running Section -->
            <div v-if="runningProjects.length > 0">
              <h3 class="text-xs font-semibold text-green-400 uppercase tracking-wider mb-3 flex items-center gap-2">
                <span class="w-2 h-2 rounded-full bg-green-400 animate-pulse"></span>
                Running ({{ runningProjects.length }})
              </h3>
              <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <Card
                  v-for="project in runningProjects"
                  :key="project.id"
                  class="cursor-pointer hover:border-primary/50 transition-colors"
                  @click="ui.selectProject(props.groupId, project.id)"
                  @contextmenu.prevent="onProjectContextMenu($event, project)"
                >
                  <CardHeader class="pb-2">
                    <div class="flex items-center justify-between">
                      <div class="flex items-center gap-2 min-w-0">
                        <span class="text-sm font-medium text-foreground truncate">{{ project.name }}</span>
                        <Badge v-if="project.autoRestart" variant="outline" class="text-[10px]">auto</Badge>
                      </div>
                      <div class="flex items-center gap-2 flex-shrink-0">
                        <Button
                          variant="ghost"
                          size="icon"
                          class="h-7 w-7"
                          :class="getStatus(project.id) === 'running' ? 'text-destructive hover:text-destructive hover:bg-destructive/10' : 'text-green-400 hover:text-green-400 hover:bg-green-400/10'"
                          @click.stop="toggleProject(project.id)"
                        >
                          <StopIcon v-if="getStatus(project.id) === 'running'" class="h-4 w-4" />
                          <PlayIcon v-else class="h-4 w-4" />
                        </Button>
                        <StatusBadge :status="getStatus(project.id)" />
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent class="pt-0">
                    <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground mb-2">
                      <span class="flex items-center gap-1">
                        <DesktopIcon class="h-3 w-3" />
                        {{ getCpu(project.id).toFixed(1) }}%
                      </span>
                      <span class="flex items-center gap-1">
                        <LayersIcon class="h-3 w-3" />
                        {{ getMemory(project.id) }}
                      </span>
                      <span v-if="getPid(project.id)">PID: {{ getPid(project.id) }}</span>
                    </div>

                    <div class="grid grid-cols-2 gap-2 mb-3">
                      <div class="rounded p-1">
                        <span class="text-[10px] text-muted-foreground">CPU</span>
                        <SparklineChart
                          :data="getProjectSparkline(project.id).cpu"
                          color="#3b82f6"
                          fill-color="rgba(59, 130, 246, 0.1)"
                          :height="24"
                        />
                      </div>
                      <div class="rounded p-1">
                        <span class="text-[10px] text-muted-foreground">Memory</span>
                        <SparklineChart
                          :data="getProjectSparkline(project.id).mem"
                          color="#10b981"
                          fill-color="rgba(16, 185, 129, 0.1)"
                          :height="24"
                        />
                      </div>
                    </div>

                    <div v-if="getLogLines(project.id).length > 0">
                      <span class="text-[10px] text-muted-foreground">Recent Output</span>
                      <div class="mt-1 bg-gray-900 border border-border rounded p-2 max-h-24 overflow-hidden">
                        <div
                          v-for="(line, i) in getLogLines(project.id)"
                          :key="i"
                          class="text-[11px] text-muted-foreground font-mono truncate leading-tight"
                        >{{ line }}</div>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              </div>
            </div>

            <!-- Stopped Section -->
            <div v-if="stoppedProjects.length > 0">
              <h3 class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 flex items-center gap-2">
                <span class="w-2 h-2 rounded-full bg-muted-foreground"></span>
                Stopped ({{ stoppedProjects.length }})
              </h3>
              <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <Card
                  v-for="project in stoppedProjects"
                  :key="project.id"
                  class="cursor-pointer hover:border-primary/50 transition-colors"
                  @click="ui.selectProject(props.groupId, project.id)"
                  @contextmenu.prevent="onProjectContextMenu($event, project)"
                >
                  <CardHeader class="pb-2">
                    <div class="flex items-center justify-between">
                      <div class="flex items-center gap-2 min-w-0">
                        <span class="text-sm font-medium text-foreground truncate">{{ project.name }}</span>
                        <Badge v-if="project.autoRestart" variant="outline" class="text-[10px]">auto</Badge>
                      </div>
                      <div class="flex items-center gap-2 flex-shrink-0">
                        <Button
                          variant="ghost"
                          size="icon"
                          class="h-7 w-7 text-green-400 hover:text-green-400 hover:bg-green-400/10"
                          @click.stop="toggleProject(project.id)"
                        >
                          <PlayIcon class="h-4 w-4" />
                        </Button>
                        <StatusBadge :status="getStatus(project.id)" />
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent class="pt-0">
                    <div v-if="lastSessions.get(project.id)">
                      <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground mb-2">
                        <span class="flex items-center gap-1">
                          <ClockIcon class="h-3 w-3" />
                          {{ formatDate(lastSessions.get(project.id)!.startedAt) }}
                        </span>
                        <span v-if="lastSessions.get(project.id)!.endedAt">
                          Duration: {{ formatDuration(lastSessions.get(project.id)!.startedAt, lastSessions.get(project.id)!.endedAt!) }}
                        </span>
                        <span
                          v-if="lastSessions.get(project.id)!.exitStatus"
                          :class="lastSessions.get(project.id)!.exitStatus === 'errored' ? 'text-destructive' : 'text-muted-foreground'"
                        >
                          Exit: {{ lastSessions.get(project.id)!.exitStatus }}
                        </span>
                      </div>
                      <div v-if="lastMetrics.get(project.id)" class="flex items-center gap-3 text-xs text-muted-foreground mb-2">
                        <span class="flex items-center gap-1">
                          <DesktopIcon class="h-3 w-3" />
                          Last CPU: {{ lastMetrics.get(project.id)!.cpuUsage.toFixed(1) }}%
                        </span>
                        <span class="flex items-center gap-1">
                          <LayersIcon class="h-3 w-3" />
                          Last MEM: {{ formatMemory(lastMetrics.get(project.id)!.memoryUsage) }}
                        </span>
                      </div>

                      <div v-if="getLogLines(project.id).length > 0">
                        <span class="text-[10px] text-muted-foreground flex items-center gap-1">
                          <FileTextIcon class="h-3 w-3" />
                          Last Output
                        </span>
                        <div class="mt-1 bg-gray-900 border border-border rounded p-2 max-h-24 overflow-hidden">
                          <div
                            v-for="(line, i) in getLogLines(project.id)"
                            :key="i"
                            class="text-[11px] text-muted-foreground font-mono truncate leading-tight"
                          >{{ line }}</div>
                        </div>
                      </div>
                    </div>
                    <div v-else class="text-xs text-muted-foreground">No session history</div>
                  </CardContent>
                </Card>
              </div>
            </div>
          </div>
        </div>
      </TabsContent>

      <TabsContent value="task" class="m-0">
        <div class="flex-1 overflow-y-auto p-4">
          <div v-if="!group || runningProjects.length === 0 && stoppedProjects.length === 0" class="text-center text-muted-foreground py-8">
            No tasks in this group.
          </div>
          <div v-else class="space-y-6">
            <!-- Running Tasks -->
            <div v-if="runningProjects.length > 0">
              <h3 class="text-xs font-semibold text-green-400 uppercase tracking-wider mb-3 flex items-center gap-2">
                <span class="w-2 h-2 rounded-full bg-green-400 animate-pulse"></span>
                Running ({{ runningProjects.length }})
              </h3>
              <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <Card
                  v-for="project in runningProjects"
                  :key="project.id"
                  class="cursor-pointer hover:border-primary/50 transition-colors"
                  @click="ui.selectProject(props.groupId, project.id)"
                >
                  <CardHeader class="pb-2">
                    <div class="flex items-center justify-between">
                      <span class="text-sm font-medium text-foreground truncate">{{ project.name }}</span>
                      <div class="flex items-center gap-2">
                        <Button
                          variant="ghost"
                          size="icon"
                          class="h-7 w-7 text-destructive hover:text-destructive hover:bg-destructive/10"
                          @click.stop="toggleProject(project.id)"
                        >
                          <StopIcon class="h-4 w-4" />
                        </Button>
                        <StatusBadge :status="getStatus(project.id)" />
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent class="pt-0">
                    <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
                      <span class="flex items-center gap-1">
                        <DesktopIcon class="h-3 w-3" />
                        {{ getCpu(project.id).toFixed(1) }}%
                      </span>
                      <span class="flex items-center gap-1">
                        <LayersIcon class="h-3 w-3" />
                        {{ getMemory(project.id) }}
                      </span>
                    </div>
                  </CardContent>
                </Card>
              </div>
            </div>

            <!-- Completed Tasks -->
            <div v-if="stoppedProjects.length > 0">
              <h3 class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 flex items-center gap-2">
                <span class="w-2 h-2 rounded-full bg-muted-foreground"></span>
                Completed ({{ stoppedProjects.length }})
              </h3>
              <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <Card
                  v-for="project in stoppedProjects"
                  :key="project.id"
                  class="cursor-pointer hover:border-primary/50 transition-colors"
                  @click="ui.selectProject(props.groupId, project.id)"
                >
                  <CardHeader class="pb-2">
                    <div class="flex items-center justify-between">
                      <span class="text-sm font-medium text-foreground truncate">{{ project.name }}</span>
                      <StatusBadge :status="getStatus(project.id)" />
                    </div>
                  </CardHeader>
                  <CardContent class="pt-0">
                    <div v-if="lastSessions.get(project.id)" class="text-xs text-muted-foreground">
                      <span>{{ formatDate(lastSessions.get(project.id)!.startedAt) }}</span>
                    </div>
                  </CardContent>
                </Card>
              </div>
            </div>
          </div>
        </div>
      </TabsContent>
    </Tabs>

    <!-- Project Context Menu -->
    <Teleport to="body">
      <div
        v-if="showContextMenu && contextMenuProject"
        class="fixed z-50 min-w-[12rem] bg-popover border border-border rounded-lg shadow-lg py-1"
        :style="{ left: contextMenuPos.x + 'px', top: contextMenuPos.y + 'px' }"
      >
        <div class="px-3 py-1.5 text-xs text-muted-foreground border-b border-border">
          {{ contextMenuProject.name }}
        </div>
        <button
          v-if="contextMenuProject.projectType !== 'service'"
          class="w-full px-3 py-1.5 text-left text-sm hover:bg-accent transition-colors"
          @click="handleConvertProject('service')"
        >
          Convert to Service
        </button>
        <button
          v-if="contextMenuProject.projectType !== 'task'"
          class="w-full px-3 py-1.5 text-left text-sm hover:bg-accent transition-colors"
          @click="handleConvertProject('task')"
        >
          Convert to Task
        </button>
        <Separator />
        <button
          class="w-full px-3 py-1.5 text-left text-sm text-destructive hover:bg-destructive/10 transition-colors"
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
