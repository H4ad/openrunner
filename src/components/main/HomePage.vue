<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke, listen, type UnlistenFn } from "@/lib/api";
import { useConfigStore } from "../../stores/config";
import { useProcessesStore } from "../../stores/processes";
import { useLogsStore } from "../../stores/logs";
import { useUiStore } from "../../stores/ui";
import type { StorageStats, ProcessInfo } from "../../types";
import { formatBytes } from "../../utils/formatters";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import SparklineChart from "../shared/SparklineChart.vue";
import EditDialog from "../shared/EditDialog.vue";
import {
  ActivityLogIcon,
  DesktopIcon,
  GearIcon,
  HomeIcon,
  LayersIcon,
  PlayIcon,
  PlusIcon,
  StopIcon,
  UploadIcon,
} from "@radix-icons/vue";
import { open } from "@/lib/dialog";
import { RefreshCcw } from "lucide-vue-next";

const config = useConfigStore();
const processes = useProcessesStore();
const logsStore = useLogsStore();
const ui = useUiStore();

const storageStats = ref<StorageStats | null>(null);
const loadingStorage = ref(false);
const startStopLoading = ref<Record<string, boolean>>({});
const globalSparkline = ref<{ cpu: number[]; mem: number[] }>({ cpu: [], mem: [] });
const showNewGroupDialog = ref(false);

const MAX_SPARKLINE = 40;
let unlisten: UnlistenFn | null = null;

const serviceCount = computed(() =>
  config.groups.reduce(
    (total, group) => total + group.projects.filter((p) => p.projectType === "service").length,
    0,
  ),
);

const groupCount = computed(() => config.groups.length);

const runningProjects = computed(() =>
  config.groups
    .flatMap((group) => group.projects)
    .filter((project) => processes.getStatus(project.id)?.status === "running"),
);

const erroredProjects = computed(() =>
  config.groups
    .flatMap((group) => group.projects)
    .filter((project) => processes.getStatus(project.id)?.status === "errored"),
);

const stoppedProjects = computed(() =>
  config.groups
    .flatMap((group) => group.projects)
    .filter((project) => {
      const status = processes.getStatus(project.id)?.status ?? "stopped";
      return status === "stopped";
    }),
);

const servicesRunning = computed(() =>
  runningProjects.value.filter((project) => project.projectType === "service").length,
);

const tasksRunning = computed(() =>
  runningProjects.value.filter((project) => project.projectType === "task").length,
);

const globalCpuAvg = computed(() => {
  const running = runningProjects.value;
  if (running.length === 0) return 0;
  const total = running.reduce((sum, project) => {
    const info = processes.getStatus(project.id);
    return sum + (info?.cpuUsage ?? 0);
  }, 0);
  return total / running.length;
});

const globalMemoryTotal = computed(() =>
  runningProjects.value.reduce((sum, project) => {
    const info = processes.getStatus(project.id);
    return sum + (info?.memoryUsage ?? 0);
  }, 0),
);

// Filter out PTY-enabled projects from recent activity
const recentActivity = computed(() => {
  const items: { projectName: string; groupName: string; message: string; timestamp: number }[] = [];
  for (const group of config.groups) {
    for (const project of group.projects) {
      if (project.interactive) continue;
      const logs = logsStore.getProjectLogs(project.id);
      if (!logs.length) continue;
      const recentLogs = logs.slice(-5);
      for (const entry of recentLogs) {
        items.push({
          projectName: project.name,
          groupName: group.name,
          message: entry.data.trim(),
          timestamp: entry.timestamp,
        });
      }
    }
  }
  return items
    .filter((item) => item.message.length > 0)
    .sort((a, b) => b.timestamp - a.timestamp)
    .slice(0, 6);
});

// Group helpers — services only
function getGroupServices(groupId: string) {
  const group = config.groups.find((g) => g.id === groupId);
  return group?.projects.filter((p) => p.projectType === "service") ?? [];
}

function getGroupServiceStatusCount(groupId: string, status: "running" | "stopped" | "errored") {
  return getGroupServices(groupId).filter((project) => {
    const current = processes.getStatus(project.id)?.status ?? "stopped";
    return current === status;
  }).length;
}

function isAnyServiceRunning(groupId: string) {
  return getGroupServiceStatusCount(groupId, "running") > 0;
}

function getGroupCpuAvg(groupId: string) {
  const running = getGroupServices(groupId).filter(
    (project) => processes.getStatus(project.id)?.status === "running",
  );
  if (!running.length) return 0;
  const total = running.reduce((sum, project) => {
    const info = processes.getStatus(project.id);
    return sum + (info?.cpuUsage ?? 0);
  }, 0);
  return total / running.length;
}

function getGroupMemoryTotal(groupId: string) {
  return getGroupServices(groupId).reduce((sum, project) => {
    const info = processes.getStatus(project.id);
    return sum + (info?.memoryUsage ?? 0);
  }, 0);
}

async function loadStorageStats() {
  loadingStorage.value = true;
  try {
    storageStats.value = await invoke<StorageStats>("get_storage_stats");
  } catch {
    storageStats.value = null;
  } finally {
    loadingStorage.value = false;
  }
}

async function handleCreateGroup(name: string, directory?: string) {
  await config.createGroup(name, directory ?? ".");
  showNewGroupDialog.value = false;
}

async function startAllInGroup(groupId: string) {
  const services = getGroupServices(groupId);
  if (!services.length) return;
  startStopLoading.value[groupId] = true;
  try {
    await processes.startAllInGroup(groupId, services.map((p) => p.id));
  } finally {
    startStopLoading.value[groupId] = false;
  }
}

async function stopAllInGroup(groupId: string) {
  const services = getGroupServices(groupId);
  if (!services.length) return;
  startStopLoading.value[groupId] = true;
  try {
    await processes.stopAllInGroup(services.map((p) => p.id));
  } finally {
    startStopLoading.value[groupId] = false;
  }
}

function openGroupMonitor(groupId: string) {
  ui.showGroupMonitor(groupId);
}

async function importGroup() {
  const filePath = await open({
    filters: [
      { name: "YAML", extensions: ["yaml", "yml"] },
      { name: "All Files", extensions: ["*"] },
    ],
    multiple: false,
  });
  if (filePath) {
    try {
      await config.importGroup(filePath as string);
    } catch (e) {
      console.error("Failed to import group:", e);
    }
  }
}

function formatTimeAgo(timestamp: number) {
  const diff = Date.now() - timestamp;
  if (diff < 60000) return "just now";
  const minutes = Math.floor(diff / 60000);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

onMounted(async () => {
  await loadStorageStats();
  unlisten = await listen<ProcessInfo[]>("process-stats-updated", () => {
    const running = runningProjects.value;
    if (!running.length) return;
    const cpu = globalCpuAvg.value;
    const mem = globalMemoryTotal.value / (1024 * 1024);
    const nextCpu = [...globalSparkline.value.cpu, parseFloat(cpu.toFixed(1))];
    const nextMem = [...globalSparkline.value.mem, parseFloat(mem.toFixed(1))];
    if (nextCpu.length > MAX_SPARKLINE) {
      nextCpu.shift();
      nextMem.shift();
    }
    globalSparkline.value = { cpu: nextCpu, mem: nextMem };
  });
});

onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <div class="flex-1 flex flex-col h-full min-h-0 bg-background">
    <!-- Header -->
    <div class="p-6 border-b border-border flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="h-10 w-10 rounded-xl bg-primary/10 border border-primary/20 flex items-center justify-center">
          <HomeIcon class="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 class="text-xl font-semibold text-foreground">Home</h2>
          <p class="text-xs text-muted-foreground">Overview of groups, health, and recent activity</p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" class="gap-1.5" @click="config.loadGroups()">
          <RefreshCcw class="h-4 w-4" />
          Refresh
        </Button>
        <Button variant="secondary" size="sm" class="gap-1" @click="ui.showSettings()">
          <GearIcon class="h-4 w-4" />
          Settings
        </Button>
      </div>
    </div>

    <!-- Main scrollable content -->
    <div class="flex-1 overflow-y-auto">
      <div class="p-6 space-y-6">
        <!-- Health + Storage: compact horizontal row -->
        <div class="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-8 gap-3">
          <div class="rounded-lg border border-border p-3 flex flex-col">
            <span class="text-[10px] uppercase text-muted-foreground">Running</span>
            <span class="text-lg font-semibold text-green-400">{{ runningProjects.length }}</span>
          </div>
          <div class="rounded-lg border border-border p-3 flex flex-col">
            <span class="text-[10px] uppercase text-muted-foreground">Stopped</span>
            <span class="text-lg font-semibold text-foreground">{{ stoppedProjects.length }}</span>
          </div>
          <div class="rounded-lg border border-border p-3 flex flex-col">
            <span class="text-[10px] uppercase text-muted-foreground">Errored</span>
            <span class="text-lg font-semibold"
              :class="erroredProjects.length > 0 ? 'text-destructive' : 'text-foreground'">{{ erroredProjects.length
              }}</span>
          </div>
          <div class="rounded-lg border border-border p-3 flex flex-col">
            <span class="text-[10px] uppercase text-muted-foreground">Groups</span>
            <span class="text-lg font-semibold text-foreground">{{ groupCount }}</span>
          </div>
          <div class="rounded-lg border border-border p-3 flex flex-col">
            <span class="text-[10px] uppercase text-muted-foreground">Services</span>
            <span class="text-lg font-semibold text-foreground">{{ serviceCount }}</span>
          </div>
          <div class="rounded-lg border border-border p-3 flex flex-col">
            <span class="text-[10px] uppercase text-muted-foreground">DB Size</span>
            <span class="text-lg font-semibold text-foreground">{{ formatBytes(storageStats?.totalSize ?? 0) }}</span>
          </div>
          <div class="rounded-lg border border-border p-3 flex flex-col">
            <span class="text-[10px] uppercase text-muted-foreground">Sessions</span>
            <span class="text-lg font-semibold text-foreground">{{ storageStats?.sessionCount ?? 0 }}</span>
          </div>
          <div class="rounded-lg border border-border p-3 flex flex-col">
            <span class="text-[10px] uppercase text-muted-foreground">Log Entries</span>
            <span class="text-lg font-semibold text-foreground">{{ (storageStats?.logCount ?? 0).toLocaleString()
              }}</span>
          </div>
        </div>

        <Card>
          <CardHeader class="pb-3">
            <CardTitle class="text-sm font-semibold">Global Metrics</CardTitle>
          </CardHeader>
          <CardContent>
            <div class="grid grid-cols-2 gap-4">
              <div class="rounded-lg border border-border p-3">
                <div class="text-xs text-muted-foreground flex items-center gap-1">
                  <DesktopIcon class="h-3 w-3" /> CPU Avg
                </div>
                <div class="text-lg font-semibold">{{ globalCpuAvg.toFixed(1) }}%</div>
                <SparklineChart :data="globalSparkline.cpu" color="#3b82f6" fill-color="rgba(59, 130, 246, 0.1)"
                  :height="28" />
              </div>
              <div class="rounded-lg border border-border p-3">
                <div class="text-xs text-muted-foreground flex items-center gap-1">
                  <LayersIcon class="h-3 w-3" /> Memory Total
                </div>
                <div class="text-lg font-semibold">{{ formatBytes(globalMemoryTotal) }}</div>
                <SparklineChart :data="globalSparkline.mem" color="#10b981" fill-color="rgba(16, 185, 129, 0.1)"
                  :height="28" />
              </div>
            </div>
          </CardContent>
        </Card>

        <!-- Groups Overview — services only -->
        <Card>
          <CardHeader class="pb-2 flex flex-row items-center justify-between">
            <div>
              <CardTitle class="text-sm font-semibold">Groups Overview</CardTitle>
              <p class="text-xs text-muted-foreground">{{ groupCount }} groups · {{ serviceCount }} services</p>
            </div>
            <div class="flex items-center gap-2">
              <Button variant="outline" size="sm" class="gap-1.5" @click="showNewGroupDialog = true">
                <PlusIcon class="h-4 w-4" />
                Create Group
              </Button>
              <Button variant="outline" size="sm" class="gap-1.5" @click="importGroup">
                <UploadIcon class="h-4 w-4" />
                Import Group
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            <div v-if="config.groups.length === 0" class="text-sm text-muted-foreground py-6 text-center">
              No groups yet. Create or import one to get started.
            </div>
            <div v-else class="grid grid-cols-1 lg:grid-cols-2 2xl:grid-cols-3 gap-4">
              <div v-for="group in config.groups" :key="group.id"
                class="rounded-lg border border-border p-4 hover:border-primary/40 transition-colors cursor-pointer"
                @click="openGroupMonitor(group.id)">
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <div class="flex items-center gap-2">
                      <h3 class="text-sm font-semibold text-foreground">{{ group.name }}</h3>
                      <Badge variant="outline" class="text-[10px]">
                        {{ getGroupServiceStatusCount(group.id, "running") }}/{{ getGroupServices(group.id).length }}
                      </Badge>
                    </div>
                    <div class="text-[11px] text-muted-foreground truncate max-w-[220px]">
                      {{ group.directory }}
                    </div>
                  </div>
                  <div class="flex items-center gap-1" @click.stop>
                    <Button v-if="isAnyServiceRunning(group.id)" variant="destructive" size="sm" class="gap-1"
                      :disabled="startStopLoading[group.id]" @click="stopAllInGroup(group.id)">
                      <StopIcon class="h-4 w-4" /> Stop All
                    </Button>
                    <Button v-else variant="default" size="sm" class="gap-1"
                      :disabled="startStopLoading[group.id] || getGroupServices(group.id).length === 0"
                      @click="startAllInGroup(group.id)">
                      <PlayIcon class="h-4 w-4" /> Start All
                    </Button>
                  </div>
                </div>

                <div class="mt-4 grid grid-cols-3 gap-2 text-xs text-muted-foreground">
                  <div>
                    <span class="block text-[10px] uppercase">Running</span>
                    <span class="font-medium text-foreground">{{ getGroupServiceStatusCount(group.id, "running")
                      }}</span>
                  </div>
                  <div>
                    <span class="block text-[10px] uppercase">Stopped</span>
                    <span class="font-medium text-foreground">{{ getGroupServiceStatusCount(group.id, "stopped")
                      }}</span>
                  </div>
                  <div>
                    <span class="block text-[10px] uppercase">Errored</span>
                    <span class="font-medium text-foreground">{{ getGroupServiceStatusCount(group.id, "errored")
                      }}</span>
                  </div>
                </div>

                <div class="mt-4 flex items-center gap-4 text-xs text-muted-foreground">
                  <div class="flex items-center gap-1">
                    <DesktopIcon class="h-3 w-3" />
                    {{ getGroupCpuAvg(group.id).toFixed(1) }}%
                  </div>
                  <div class="flex items-center gap-1">
                    <LayersIcon class="h-3 w-3" />
                    {{ formatBytes(getGroupMemoryTotal(group.id)) }}
                  </div>
                  <div class="flex items-center gap-1">
                    <ActivityLogIcon class="h-3 w-3" />
                    {{ getGroupServices(group.id).length }} services
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <!-- Global Metrics + Recent Activity -->
        <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
          <Card>
            <CardHeader class="pb-3">
              <CardTitle class="text-sm font-semibold">Recent Activity</CardTitle>
            </CardHeader>
            <CardContent class="space-y-3">
              <div v-if="recentActivity.length === 0" class="text-xs text-muted-foreground">
                No recent logs yet.
              </div>
              <div v-else class="space-y-2">
                <div v-for="(item, index) in recentActivity" :key="index"
                  class="rounded-md border border-border/60 p-2">
                  <div class="flex items-center justify-between text-xs text-muted-foreground">
                    <span class="font-medium text-foreground">{{ item.projectName }}</span>
                    <span>{{ formatTimeAgo(item.timestamp) }}</span>
                  </div>
                  <div class="text-[11px] text-muted-foreground truncate">{{ item.groupName }}</div>
                  <div class="mt-1 text-xs text-muted-foreground truncate font-mono">
                    {{ item.message }}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

      </div>
    </div>

  </div>

  <EditDialog :open="showNewGroupDialog" title="New Group" label="Group Name" placeholder="My Project Group"
    secondary-label="Working Directory" secondary-placeholder="/path/to/workspace" secondary-browse-directory
    @confirm="handleCreateGroup" @cancel="showNewGroupDialog = false" />
</template>
