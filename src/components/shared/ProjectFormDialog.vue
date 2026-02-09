<script setup lang="ts">
import { ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectType } from "../../types";

const props = defineProps<{
  open: boolean;
  title: string;
  name?: string;
  command?: string;
  cwd?: string;
  envVars?: Record<string, string>;
  projectType?: ProjectType;
}>();

const emit = defineEmits<{
  confirm: [
    name: string,
    command: string,
    cwd: string | undefined,
    envVars: Record<string, string>,
    projectType: ProjectType,
  ];
  cancel: [];
}>();

const nameValue = ref("");
const commandValue = ref("");
const cwdValue = ref("");
const projectTypeValue = ref<ProjectType>("service");
const envRows = ref<{ key: string; value: string }[]>([]);
const showEnvVars = ref(false);

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      nameValue.value = props.name ?? "";
      commandValue.value = props.command ?? "";
      cwdValue.value = props.cwd ?? "";
      projectTypeValue.value = props.projectType ?? "service";
      const entries = Object.entries(props.envVars ?? {});
      envRows.value =
        entries.length > 0
          ? entries.map(([key, value]) => ({ key, value }))
          : [];
      showEnvVars.value = envRows.value.length > 0;
    }
  },
);

async function browseFolder() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select Working Directory",
  });
  if (selected) {
    cwdValue.value = selected as string;
  }
}

function addEnvRow() {
  envRows.value.push({ key: "", value: "" });
  showEnvVars.value = true;
}

function removeEnvRow(index: number) {
  envRows.value.splice(index, 1);
}

function submit() {
  if (!nameValue.value.trim() || !commandValue.value.trim()) return;
  const envVarsResult: Record<string, string> = {};
  for (const row of envRows.value) {
    const key = row.key.trim();
    if (key) {
      envVarsResult[key] = row.value;
    }
  }
  emit(
    "confirm",
    nameValue.value.trim(),
    commandValue.value.trim(),
    cwdValue.value.trim() || undefined,
    envVarsResult,
    projectTypeValue.value,
  );
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="emit('cancel')"
    >
      <div class="bg-gray-800 rounded-lg shadow-xl p-6 w-[28rem] border border-gray-700">
        <h3 class="text-lg font-semibold text-gray-100 mb-4">
          {{ props.title }}
        </h3>
        <form @submit.prevent="submit">
          <label class="block text-sm text-gray-400 mb-1">Project Name</label>
          <input
            v-model="nameValue"
            placeholder="My Project"
            class="w-full px-3 py-2 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500 mb-4"
            autofocus
          />

          <label class="block text-sm text-gray-400 mb-1">Command</label>
          <input
            v-model="commandValue"
            placeholder="npm run dev"
            class="w-full px-3 py-2 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500 mb-4"
          />

          <label class="block text-sm text-gray-400 mb-1">Type</label>
          <div class="flex gap-2 mb-4">
            <button
              type="button"
              class="flex-1 px-3 py-2 text-sm rounded border transition-colors"
              :class="projectTypeValue === 'service'
                ? 'bg-blue-600 border-blue-600 text-white'
                : 'bg-gray-900 border-gray-600 text-gray-400 hover:border-gray-500'"
              @click="projectTypeValue = 'service'"
            >
              Service
            </button>
            <button
              type="button"
              class="flex-1 px-3 py-2 text-sm rounded border transition-colors"
              :class="projectTypeValue === 'task'
                ? 'bg-blue-600 border-blue-600 text-white'
                : 'bg-gray-900 border-gray-600 text-gray-400 hover:border-gray-500'"
              @click="projectTypeValue = 'task'"
            >
              Task
            </button>
          </div>

          <label class="block text-sm text-gray-400 mb-1">
            Working Directory
            <span class="text-gray-600">(optional, relative to group)</span>
          </label>
          <div class="flex gap-2 mb-4">
            <input
              v-model="cwdValue"
              placeholder="packages/my-app"
              class="flex-1 px-3 py-2 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500"
            />
            <button
              type="button"
              class="px-2 py-2 text-xs rounded bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors flex-shrink-0"
              title="Browse folder"
              @click="browseFolder"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
              </svg>
            </button>
          </div>

          <!-- Environment Variables -->
          <div class="mb-4">
            <div class="flex items-center justify-between mb-1">
              <label class="text-sm text-gray-400">Environment Variables</label>
              <button
                type="button"
                class="text-xs text-blue-400 hover:text-blue-300 transition-colors"
                @click="addEnvRow"
              >
                + Add
              </button>
            </div>
            <div v-if="envRows.length > 0" class="space-y-2 max-h-32 overflow-y-auto">
              <div
                v-for="(row, index) in envRows"
                :key="index"
                class="flex items-center gap-2"
              >
                <input
                  v-model="row.key"
                  placeholder="KEY"
                  class="flex-1 px-2 py-1.5 bg-gray-900 border border-gray-600 rounded text-gray-100 text-xs focus:outline-none focus:border-blue-500 font-mono"
                />
                <input
                  v-model="row.value"
                  placeholder="value"
                  class="flex-1 px-2 py-1.5 bg-gray-900 border border-gray-600 rounded text-gray-100 text-xs focus:outline-none focus:border-blue-500 font-mono"
                />
                <button
                  type="button"
                  class="p-0.5 text-gray-500 hover:text-red-400 transition-colors flex-shrink-0"
                  @click="removeEnvRow(index)"
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>
            <p v-else class="text-xs text-gray-600">No environment variables set.</p>
          </div>

          <div class="flex justify-end gap-3">
            <button
              type="button"
              class="px-4 py-2 text-sm rounded bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors"
              @click="emit('cancel')"
            >
              Cancel
            </button>
            <button
              type="submit"
              class="px-4 py-2 text-sm rounded bg-blue-600 text-white hover:bg-blue-500 transition-colors"
              :disabled="!nameValue.trim() || !commandValue.trim()"
            >
              Save
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
