<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { Group, Project, Session, MetricPoint } from "../../types";
import type { ProjectType } from "../../types";
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
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  ActivityLogIcon,
  ClockIcon,
  ArrowLeftIcon,
  Pencil1Icon,
  TrashIcon,
  PlayIcon,
  StopIcon,
  LayersIcon,
  CodeIcon,
} from "@radix-icons/vue";

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

// Only services can be started via "Start All"
const serviceProjects = computed(() =>
  props.group.projects.filter((p) => p.projectType !== 'task')
);
const groupRunningCount = computed(() =>
  serviceProjects.value.filter((p) => processes.getStatus(p.id)?.status === "running").length,
);
const groupTotalCount = computed(() => serviceProjects.value.length);
const allRunning = computed(() => groupRunningCount.value === groupTotalCount.value && groupTotalCount.value > 0);
const someRunning = computed(() => groupRunningCount.value > 0);

const startStopAllLoading = ref(false);

async function startAllInGroup() {
  startStopAllLoading.value = true;
  try {
    await processes.startAllInGroup(
      props.group.id,
      serviceProjects.value.map((p) => p.id),
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
    const session = await sessionsStore.getLastSession(props.group.id, props.project.id);
    lastSession.value = session;
    if (session) {
      lastMetric.value = await sessionsStore.getLastMetric(props.group.id, session.id);
      lastSessionMetrics.value = await sessionsStore.getSessionMetrics(props.group.id, session.id);
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
  projectType?: string,
  interactive: boolean = false,
) {
  await config.updateProject(props.group.id, props.project.id, {
    name,
    command,
    cwd: cwd || null,
    envVars: envVars ?? {},
    projectType: (projectType || 'service') as ProjectType,
    interactive,
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
    <div class="px-4 py-2 border-b border-border bg-muted/50 flex items-center justify-between">
      <div class="flex items-center gap-2 min-w-0">
        <Button variant="ghost" size="icon" class="h-7 w-7 shrink-0" title="Back" @click="ui.clearSelection()">
          <ArrowLeftIcon class="h-3.5 w-3.5" />
        </Button>
        <LayersIcon class="h-3.5 w-3.5 text-muted-foreground shrink-0" />
        <span class="text-xs text-muted-foreground truncate">{{ props.group.name }}</span>
        <span class="text-xs text-muted-foreground">/</span>
        <CodeIcon class="h-3.5 w-3.5 text-foreground shrink-0" />
        <span class="text-xs text-foreground font-medium truncate">{{ props.project.name }}</span>
        <Badge
          :variant="someRunning ? 'default' : 'secondary'"
          class="text-[10px] shrink-0"
        >
          {{ groupRunningCount }}/{{ groupTotalCount }}
        </Badge>
      </div>
      <div class="flex items-center gap-1.5 shrink-0">
        <Button
          :variant="allRunning ? 'destructive' : 'default'"
          size="sm"
          class="text-[11px] h-7 gap-1"
          :disabled="startStopAllLoading"
          @click="allRunning ? stopAllInGroup() : startAllInGroup()"
        >
          <StopIcon v-if="allRunning" class="h-3 w-3" />
          <PlayIcon v-else class="h-3 w-3" />
          <span>{{ allRunning ? 'Stop All' : 'Start All' }}</span>
        </Button>
        <Button
          variant="ghost"
          size="icon"
          class="h-7 w-7"
          title="Group Monitor"
          @click="ui.showGroupMonitor(props.group.id)"
        >
          <ActivityLogIcon class="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>

    <!-- Header -->
    <div class="p-4 border-b border-border space-y-3">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h2 class="text-lg font-semibold text-foreground">
            {{ props.project.name }}
          </h2>
          <StatusBadge :status="status" />
        </div>
        <div class="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8"
            :class="ui.showMonitor ? 'bg-accent text-accent-foreground' : ''"
            title="Monitor"
            @click="ui.showMonitor = !ui.showMonitor"
          >
            <ActivityLogIcon class="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8"
            title="Sessions"
            @click="showSessions = true"
          >
            <ClockIcon class="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8"
            title="Edit Project"
            @click="showEditDialog = true"
          >
            <Pencil1Icon class="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8 text-destructive hover:text-destructive hover:bg-destructive/10"
            title="Delete Project"
            @click="showDeleteDialog = true"
          >
            <TrashIcon class="h-4 w-4" />
          </Button>
        </div>
      </div>

      <div class="text-xs font-mono bg-muted px-3 py-2 rounded border border-border">
        <span class="text-foreground">{{ props.project.command }}</span>
        <span v-if="props.project.cwd" class="text-muted-foreground ml-2">cwd: {{ props.project.cwd }}</span>
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
      <div v-else-if="lastSession" class="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
        <div class="flex items-center gap-1">
          <ClockIcon class="h-3.5 w-3.5" />
          <span>Last run: {{ formatDate(lastSession.startedAt) }}</span>
        </div>
        <div v-if="lastSession.endedAt" class="flex items-center gap-1">
          <span>Duration: {{ formatDuration(lastSession.startedAt, lastSession.endedAt) }}</span>
        </div>
        <div v-if="lastSession.exitStatus" class="flex items-center gap-1">
          <span :class="lastSession.exitStatus === 'errored' ? 'text-destructive' : 'text-muted-foreground'">
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
      :group-id="props.group.id"
      @close="showSessions = false"
    />
    <LogPanel
      v-else
      :project-id="props.project.id"
      :group-id="props.group.id"
      :interactive="props.project.interactive"
    />

    <!-- Dialogs -->
    <ProjectFormDialog
      :open="showEditDialog"
      title="Edit Project"
      :name="props.project.name"
      :command="props.project.command"
      :cwd="props.project.cwd ?? undefined"
      :env-vars="props.project.envVars"
      :project-type="props.project.projectType"
      :interactive="props.project.interactive"
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
