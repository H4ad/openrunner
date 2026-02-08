<script setup lang="ts">
import { ref, computed } from "vue";
import type { Group } from "../../types";
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
const showRenameDialog = ref(false);
const showDeleteDialog = ref(false);
const showAddProjectDialog = ref(false);
const showEditDirectoryDialog = ref(false);
const showEnvVarsDialog = ref(false);

const isExpanded = () => ui.isGroupExpanded(props.group.id);

function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  contextMenuPos.value = { x: e.clientX, y: e.clientY };
  showContextMenu.value = true;

  const closeMenu = () => {
    showContextMenu.value = false;
    document.removeEventListener("click", closeMenu);
  };
  setTimeout(() => document.addEventListener("click", closeMenu), 0);
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
  <div>
    <div class="flex items-center gap-0.5">
      <button
        class="flex-1 min-w-0 text-left px-2 py-1.5 text-sm font-medium text-gray-300 hover:bg-gray-700/50 rounded flex items-center gap-1 transition-colors"
        @click="ui.toggleGroup(props.group.id)"
        @contextmenu="onContextMenu"
      >
        <svg
          class="w-3 h-3 transition-transform flex-shrink-0"
          :class="isExpanded() ? 'rotate-90' : ''"
          fill="currentColor"
          viewBox="0 0 20 20"
        >
          <path
            fill-rule="evenodd"
            d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
            clip-rule="evenodd"
          />
        </svg>
        <span class="truncate">{{ props.group.name }}</span>
      </button>
      <button
        v-if="totalCount > 0"
        class="flex-shrink-0 flex items-center gap-0.5 px-1.5 py-1 rounded text-[10px] transition-colors"
        :class="runningCount > 0 ? 'text-green-400 hover:bg-green-900/30' : 'text-gray-500 hover:bg-gray-700/50'"
        title="Group Monitor"
        @click.stop="ui.showGroupMonitor(props.group.id)"
      >
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
        </svg>
        <span>{{ runningCount }}/{{ totalCount }}</span>
      </button>
    </div>

    <!-- Context menu -->
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

    <!-- Expanded projects -->
    <div v-if="isExpanded()" class="ml-3 mt-0.5 space-y-0.5">
      <ProjectItem
        v-for="project in props.group.projects"
        :key="project.id"
        :project="project"
        :group-id="props.group.id"
      />
      <button
        v-if="props.group.projects.length === 0"
        class="w-full text-left px-3 py-1.5 text-xs text-gray-500 hover:text-gray-400"
        @click="showAddProjectDialog = true"
      >
        + Add a project
      </button>
    </div>

    <!-- Dialogs -->
    <EditDialog
      :open="showRenameDialog"
      title="Rename Group"
      label="Group Name"
      :value="props.group.name"
      @confirm="handleRename"
      @cancel="showRenameDialog = false"
    />

    <EditDialog
      :open="showEditDirectoryDialog"
      title="Change Directory"
      label="Working Directory"
      :value="props.group.directory"
      placeholder="/path/to/directory"
      browse-directory
      @confirm="handleEditDirectory"
      @cancel="showEditDirectoryDialog = false"
    />

    <ProjectFormDialog
      :open="showAddProjectDialog"
      title="Add Project"
      @confirm="handleAddProject"
      @cancel="showAddProjectDialog = false"
    />

    <EnvVarsEditor
      :open="showEnvVarsDialog"
      :title="`Environment Variables - ${props.group.name}`"
      :env-vars="props.group.envVars"
      @confirm="handleUpdateEnvVars"
      @cancel="showEnvVarsDialog = false"
    />

    <ConfirmDialog
      :open="showDeleteDialog"
      title="Delete Group"
      :message="`Are you sure you want to delete '${props.group.name}' and all its projects?`"
      @confirm="handleDelete"
      @cancel="showDeleteDialog = false"
    />
  </div>
</template>
