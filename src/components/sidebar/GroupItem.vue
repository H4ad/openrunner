<script setup lang="ts">
import { ref, computed } from "vue";
import type { Group, ProjectType } from "../../types";
import { useUiStore } from "../../stores/ui";
import { useConfigStore } from "../../stores/config";
import { useProcessesStore } from "../../stores/processes";
import ProjectItem from "./ProjectItem.vue";
import EditDialog from "../shared/EditDialog.vue";
import ProjectFormDialog from "../shared/ProjectFormDialog.vue";
import ConfirmDialog from "../shared/ConfirmDialog.vue";
import EnvVarsEditor from "../shared/EnvVarsEditor.vue";
import { invoke } from "@tauri-apps/api/core";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  LayersIcon,
  PlayIcon,
  StopIcon,
  PlusIcon,
  ChevronRightIcon,
  ActivityLogIcon,
  FileTextIcon,
  CodeIcon,
  Pencil1Icon,
  TrashIcon,
} from "@radix-icons/vue";

const props = defineProps<{
  group: Group;
}>();

const ui = useUiStore();
const config = useConfigStore();
const processesStore = useProcessesStore();

const totalCount = computed(() => props.group.projects.length);
const runningCount = computed(
  () =>
    props.group.projects.filter(
      (p) => processesStore.getStatus(p.id)?.status === "running",
    ).length,
);

const showContextMenu = ref(false);
const contextMenuPos = ref({ x: 0, y: 0 });
const showProjectContextMenu = ref(false);
const projectContextMenuPos = ref({ x: 0, y: 0 });
const showRenameDialog = ref(false);
const showDeleteDialog = ref(false);
const showAddProjectDialog = ref(false);
const showEditDirectoryDialog = ref(false);
const showEnvVarsDialog = ref(false);
const showDeleteSelectedDialog = ref(false);

// Tab filtering
const selectedTab = ref<"all" | "service" | "task">("service");

const filteredProjects = computed(() => {
  if (selectedTab.value === "all") {
    return props.group.projects;
  }
  return props.group.projects.filter((p) => p.projectType === selectedTab.value);
});

const selectedProjectsInGroup = computed(() =>
  ui.selectedProjects.filter((s) => s.groupId === props.group.id),
);

const selectedProjectIds = computed(() =>
  selectedProjectsInGroup.value.map((s) => s.projectId),
);

const hasSelectedProjects = computed(() => selectedProjectIds.value.length > 0);

function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  contextMenuPos.value = { x: e.clientX, y: e.clientY };
  showContextMenu.value = true;
  document.addEventListener("click", () => {
    showContextMenu.value = false;
  }, { once: true });
}

function onProjectContextMenu(e: MouseEvent) {
  e.preventDefault();
  projectContextMenuPos.value = { x: e.clientX, y: e.clientY };
  showProjectContextMenu.value = true;
  document.addEventListener("click", () => {
    showProjectContextMenu.value = false;
  }, { once: true });
}

async function handleDeleteSelected() {
  await config.deleteMultipleProjects(props.group.id, selectedProjectIds.value);
  ui.clearProjectSelection();
  showDeleteSelectedDialog.value = false;
}

async function handleConvertSelectedTo(type: "service" | "task") {
  await config.convertMultipleProjects(props.group.id, selectedProjectIds.value, type);
  ui.clearProjectSelection();
  showProjectContextMenu.value = false;
}

async function handleRename(name: string) {
  await config.renameGroup(props.group.id, name);
  showRenameDialog.value = false;
}

async function handleEditDirectory(directory: string) {
  await config.updateGroupDirectory(props.group.id, directory);
  showEditDirectoryDialog.value = false;
}

async function handleDelete() {
  await config.deleteGroup(props.group.id);
  showDeleteDialog.value = false;
  if (ui.selectedGroupId === props.group.id) {
    ui.clearSelection();
  }
}

async function handleUpdateEnvVars(envVars: Record<string, string>) {
  await config.updateGroupEnvVars(props.group.id, envVars);
  showEnvVarsDialog.value = false;
}

async function handleAddProject(name: string, command: string, cwd?: string) {
  const project = await config.createProject(props.group.id, name, command, cwd);
  showAddProjectDialog.value = false;
  ui.selectProject(props.group.id, project.id);
}

async function startGroup() {
  const serviceProjects = props.group.projects.filter(p => p.projectType === "service");
  for (const project of serviceProjects) {
    const status = processesStore.getStatus(project.id)?.status;
    if (status !== "running" && status !== "stopping") {
      await processesStore.startProcess(props.group.id, project.id);
    }
  }
}

async function stopGroup() {
  const runningProjects = props.group.projects.filter(
    p => processesStore.getStatus(p.id)?.status === "running"
  );
  for (const project of runningProjects) {
    await processesStore.stopProcess(project.id);
  }
}

async function openFolder() {
  try {
    await invoke("open_path", { path: props.group.directory });
  } catch (e) {
    console.error("Failed to open folder:", e);
  }
}

async function openInTerminal() {
  try {
    await invoke("open_in_terminal", { path: props.group.directory });
  } catch (e) {
    console.error("Failed to open terminal:", e);
  }
}
</script>

<template>
  <div class="select-none">
    <Collapsible
      :open="ui.isGroupExpanded(group.id)"
      @update:open="ui.toggleGroup(group.id)">
      <!-- Group Header -->
      <CollapsibleTrigger as-child>
        <div
          class="group flex items-center gap-1 px-2 py-1.5 text-sm cursor-pointer transition-colors rounded"
          :class="[
            ui.selectedGroupId === props.group.id
              ? 'bg-accent text-foreground'
              : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground',
          ]"
          @click="ui.selectGroup(props.group.id)"
          @contextmenu.prevent="onContextMenu"
        >
          <ChevronRightIcon
            class="h-3.5 w-3.5 transition-transform duration-150"
            :class="{ 'rotate-90': ui.isGroupExpanded(group.id) }"
          />
          <LayersIcon class="h-4 w-4" />
          <span class="font-medium flex-1 min-w-0 truncate">{{ props.group.name }}</span>
          <!-- Monitor button -->
          <Badge
            v-if="totalCount > 0"
            variant="outline"
            class="flex-shrink-0 flex items-center gap-0.5 text-[10px] h-5 px-1 cursor-pointer"
            :class="runningCount > 0 ? 'border-green-500/50 text-green-400 hover:bg-green-500/10' : 'border-muted text-muted-foreground hover:bg-accent'"
            title="Group Monitor"
            @click.stop="ui.showGroupMonitor(props.group.id)"
          >
            <ActivityLogIcon class="h-3 w-3" />
            <span>{{ runningCount }}/{{ totalCount }}</span>
          </Badge>
          <Button
            v-if="runningCount > 0"
            variant="ghost"
            size="icon"
            class="h-6 w-6 opacity-0 group-hover:opacity-100"
            @click.stop="stopGroup"
          >
            <StopIcon class="h-3.5 w-3.5" />
          </Button>
          <Button
            v-else
            variant="ghost"
            size="icon"
            class="h-6 w-6 opacity-0 group-hover:opacity-100"
            @click.stop="startGroup"
          >
            <PlayIcon class="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6 opacity-0 group-hover:opacity-100"
            @click.stop="showAddProjectDialog = true"
          >
            <PlusIcon class="h-3.5 w-3.5" />
          </Button>
        </div>
      </CollapsibleTrigger>

      <!-- Projects List -->
      <CollapsibleContent>
        <div class="ml-4 pt-2">
          <!-- Service/Task Tabs -->
          <div v-if="props.group.projects.length > 0" class="flex gap-1 mb-2 px-1">
            <Button
              variant="ghost"
              size="sm"
              class="text-xs h-6 px-2"
              :class="selectedTab === 'all' ? 'bg-accent text-foreground' : 'text-muted-foreground hover:text-foreground'"
              @click="selectedTab = 'all'"
            >
              All
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="text-xs h-6 px-2"
              :class="selectedTab === 'service' ? 'bg-accent text-foreground' : 'text-muted-foreground hover:text-foreground'"
              @click="selectedTab = 'service'"
            >
              Services
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="text-xs h-6 px-2"
              :class="selectedTab === 'task' ? 'bg-accent text-foreground' : 'text-muted-foreground hover:text-foreground'"
              @click="selectedTab = 'task'"
            >
              Tasks
            </Button>
          </div>
          <div class="space-y-0.5">
            <ProjectItem
              v-for="project in filteredProjects"
              :key="project.id"
              :project="project"
              :group-id="props.group.id"
              @contextmenu="onProjectContextMenu"
            />
            <div
              v-if="filteredProjects.length === 0 && props.group.projects.length > 0"
              class="px-3 py-2 text-xs text-muted-foreground italic"
            >
              No {{ selectedTab }}s in this group
            </div>
          </div>
          <Button
            v-if="props.group.projects.length === 0"
            variant="ghost"
            size="sm"
            class="w-full justify-start text-xs text-muted-foreground hover:text-foreground"
            @click="showAddProjectDialog = true"
          >
            <PlusIcon class="h-3 w-3 mr-1" />
            Add a project
          </Button>
        </div>
      </CollapsibleContent>
    </Collapsible>

    <!-- Group Context Menu -->
    <Teleport to="body">
      <div
        v-if="showContextMenu"
        class="fixed z-50 bg-popover border border-border rounded-md shadow-lg py-1 min-w-40"
        :style="{ left: contextMenuPos.x + 'px', top: contextMenuPos.y + 'px' }"
      >
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-accent flex items-center gap-2"
          @click="showAddProjectDialog = true"
        >
          <PlusIcon class="h-3.5 w-3.5" />
          Add Project
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-accent flex items-center gap-2"
          @click="ui.showGroupMonitor(props.group.id)"
        >
          <ActivityLogIcon class="h-3.5 w-3.5" />
          Monitor
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-accent flex items-center gap-2"
          @click="showRenameDialog = true"
        >
          <Pencil1Icon class="h-3.5 w-3.5" />
          Rename Group
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-accent flex items-center gap-2"
          @click="showEditDirectoryDialog = true"
        >
          <LayersIcon class="h-3.5 w-3.5" />
          Change Directory
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-accent flex items-center gap-2"
          @click="openFolder"
        >
          <FileTextIcon class="h-3.5 w-3.5" />
          Open Folder
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-accent flex items-center gap-2"
          @click="openInTerminal"
        >
          <CodeIcon class="h-3.5 w-3.5" />
          Open in Terminal
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-accent flex items-center gap-2"
          @click="showEnvVarsDialog = true"
        >
          Environment Variables
        </button>
        <Separator class="my-1" />
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-destructive hover:bg-destructive/10 flex items-center gap-2"
          @click="showDeleteDialog = true"
        >
          <TrashIcon class="h-3.5 w-3.5" />
          Delete Group
        </button>
      </div>
    </Teleport>

    <!-- Project Selection Context Menu -->
    <Teleport to="body">
      <div
        v-if="showProjectContextMenu"
        class="fixed z-50 min-w-[12rem] bg-popover border border-border rounded-md shadow-lg py-1"
        :style="{ left: projectContextMenuPos.x + 'px', top: projectContextMenuPos.y + 'px' }"
      >
        <div
          v-if="hasSelectedProjects"
          class="px-3 py-1.5 text-xs text-muted-foreground border-b border-border"
        >
          {{ selectedProjectIds.length }} selected
        </div>
        <template v-if="hasSelectedProjects">
          <button
            class="w-full px-3 py-1.5 text-left text-sm text-foreground hover:bg-accent transition-colors"
            @click="handleConvertSelectedTo('service')"
          >
            Convert to Service
          </button>
          <button
            class="w-full px-3 py-1.5 text-left text-sm text-foreground hover:bg-accent transition-colors"
            @click="handleConvertSelectedTo('task')"
          >
            Convert to Task
          </button>
          <Separator class="my-1" />
          <button
            class="w-full px-3 py-1.5 text-left text-sm text-destructive hover:bg-destructive/10 transition-colors"
            @click="showDeleteSelectedDialog = true"
          >
            Delete Selected
          </button>
        </template>
        <template v-else>
          <div class="px-3 py-1.5 text-sm text-muted-foreground">
            Right-click on a project to select multiple with Shift+click
          </div>
        </template>
      </div>
    </Teleport>

    <!-- Dialogs -->
    <EditDialog
      :open="showRenameDialog"
      title="Rename Group"
      label="Group name"
      :value="props.group.name"
      placeholder="Group name"
      @confirm="handleRename"
      @cancel="showRenameDialog = false"
    />
    <EditDialog
      :open="showEditDirectoryDialog"
      title="Edit Directory"
      label="Directory path"
      :value="props.group.directory"
      placeholder="Directory path"
      @confirm="handleEditDirectory"
      @cancel="showEditDirectoryDialog = false"
    />
    <ConfirmDialog
      :open="showDeleteDialog"
      title="Delete Group"
      :message="`Delete '${props.group.name}' and all its projects?`"
      @confirm="handleDelete"
      @cancel="showDeleteDialog = false"
    />
    <ConfirmDialog
      :open="showDeleteSelectedDialog"
      title="Delete Selected Projects"
      :message="`Delete ${selectedProjectIds.length} selected project${selectedProjectIds.length === 1 ? '' : 's'}?`"
      @confirm="handleDeleteSelected"
      @cancel="showDeleteSelectedDialog = false"
    />
    <ProjectFormDialog
      :open="showAddProjectDialog"
      title="Add Project"
      @confirm="handleAddProject"
      @cancel="showAddProjectDialog = false"
    />
    <EnvVarsEditor
      :open="showEnvVarsDialog"
      title="Group Environment Variables"
      :env-vars="props.group.envVars"
      @confirm="handleUpdateEnvVars"
      @cancel="showEnvVarsDialog = false"
    />
  </div>
</template>
