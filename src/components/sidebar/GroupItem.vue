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

const isExpanded = () => ui.isGroupExpanded(props.group.id);

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
    <!-- Group Header -->
    <div
      class="group flex items-center gap-1 px-2 py-1.5 text-sm cursor-pointer transition-colors"
      :class="[
        ui.selectedGroupId === props.group.id
          ? 'bg-gray-700 text-gray-100'
          : 'text-gray-400 hover:bg-gray-700/50 hover:text-gray-300',
      ]"
      @click="ui.toggleGroup(props.group.id)"
      @contextmenu.prevent="onContextMenu"
    >
      <span
        class="transition-transform duration-150"
        :class="{ 'rotate-90': isExpanded() }"
      >
        <svg
          class="w-3.5 h-3.5"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M9 5l7 7-7 7"
          />
        </svg>
      </span>
      <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
        />
      </svg>
      <span class="font-medium flex-1 min-w-0 truncate">{{ props.group.name }}</span>
      <!-- Monitor button -->
      <button
        v-if="totalCount > 0"
        class="flex-shrink-0 flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] transition-colors"
        :class="runningCount > 0 ? 'text-green-400 hover:bg-green-900/30' : 'text-gray-500 hover:bg-gray-700/50'"
        title="Group Monitor"
        @click.stop="ui.showGroupMonitor(props.group.id)"
      >
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
        </svg>
        <span>{{ runningCount }}/{{ totalCount }}</span>
      </button>
      <button
        v-if="runningCount > 0"
        class="opacity-0 group-hover:opacity-100 p-1 hover:bg-gray-600 rounded transition-all"
        @click.stop="stopGroup"
      >
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <rect x="6" y="6" width="12" height="12" stroke-width="2" />
        </svg>
      </button>
      <button
        v-else
        class="opacity-0 group-hover:opacity-100 p-1 hover:bg-gray-600 rounded transition-all"
        @click.stop="startGroup"
      >
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <polygon points="5 3 19 12 5 21 5 3" stroke-width="2" fill="currentColor" />
        </svg>
      </button>
      <button
        class="opacity-0 group-hover:opacity-100 p-1 hover:bg-gray-600 rounded transition-all"
        @click.stop="showAddProjectDialog = true"
      >
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
      </button>
    </div>

    <!-- Projects List -->
    <div v-if="isExpanded()" class="ml-4 pt-2">
      <!-- Service/Task Tabs -->
      <div v-if="props.group.projects.length > 0" class="flex gap-1 mb-2 px-1">
        <button
          class="px-2 py-0.5 text-xs rounded transition-colors"
          :class="selectedTab === 'all' ? 'bg-gray-600 text-gray-100' : 'text-gray-500 hover:text-gray-300 hover:bg-gray-700/50'"
          @click="selectedTab = 'all'"
        >
          All
        </button>
        <button
          class="px-2 py-0.5 text-xs rounded transition-colors"
          :class="selectedTab === 'service' ? 'bg-gray-600 text-gray-100' : 'text-gray-500 hover:text-gray-300 hover:bg-gray-700/50'"
          @click="selectedTab = 'service'"
        >
          Services
        </button>
        <button
          class="px-2 py-0.5 text-xs rounded transition-colors"
          :class="selectedTab === 'task' ? 'bg-gray-600 text-gray-100' : 'text-gray-500 hover:text-gray-300 hover:bg-gray-700/50'"
          @click="selectedTab = 'task'"
        >
          Tasks
        </button>
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
          class="px-3 py-2 text-xs text-gray-500 italic"
        >
          No {{ selectedTab }}s in this group
        </div>
      </div>
      <button
        v-if="props.group.projects.length === 0"
        class="w-full text-left px-3 py-1.5 text-xs text-gray-500 hover:text-gray-300 transition-colors"
        @click="showAddProjectDialog = true"
      >
        + Add a project
      </button>
    </div>

    <!-- Group Context Menu -->
    <Teleport to="body">
      <div
        v-if="showContextMenu"
        class="fixed z-50 bg-gray-800 border border-gray-600 rounded shadow-lg py-1 min-w-40"
        :style="{ left: contextMenuPos.x + 'px', top: contextMenuPos.y + 'px' }"
      >
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-700"
          @click="showAddProjectDialog = true"
        >
          Add Project
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-700"
          @click="ui.showGroupMonitor(props.group.id)"
        >
          Monitor
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-700"
          @click="showRenameDialog = true"
        >
          Rename Group
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-700"
          @click="showEditDirectoryDialog = true"
        >
          Change Directory
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-700"
          @click="openFolder"
        >
          Open Folder
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-700"
          @click="openInTerminal"
        >
          Open in Terminal
        </button>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-700"
          @click="showEnvVarsDialog = true"
        >
          Environment Variables
        </button>
        <hr class="border-gray-700 my-1" />
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-red-400 hover:bg-gray-700"
          @click="showDeleteDialog = true"
        >
          Delete Group
        </button>
      </div>
    </Teleport>

    <!-- Project Selection Context Menu -->
    <Teleport to="body">
      <div
        v-if="showProjectContextMenu"
        class="fixed z-50 min-w-[12rem] bg-gray-800 border border-gray-700 rounded-lg shadow-xl py-1"
        :style="{ left: projectContextMenuPos.x + 'px', top: projectContextMenuPos.y + 'px' }"
      >
      <div
        v-if="hasSelectedProjects"
        class="px-3 py-1.5 text-xs text-gray-500 border-b border-gray-700"
      >
        {{ selectedProjectIds.length }} selected
      </div>
      <template v-if="hasSelectedProjects">
        <button
          class="w-full px-3 py-1.5 text-left text-sm text-gray-300 hover:bg-gray-700 transition-colors"
          @click="handleConvertSelectedTo('service')"
        >
          Convert to Service
        </button>
        <button
          class="w-full px-3 py-1.5 text-left text-sm text-gray-300 hover:bg-gray-700 transition-colors"
          @click="handleConvertSelectedTo('task')"
        >
          Convert to Task
        </button>
        <div class="border-t border-gray-700 my-1"></div>
        <button
          class="w-full px-3 py-1.5 text-left text-sm text-red-400 hover:bg-gray-700 transition-colors"
          @click="showDeleteSelectedDialog = true"
        >
          Delete Selected
        </button>
      </template>
      <template v-else>
        <div class="px-3 py-1.5 text-sm text-gray-500">
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
