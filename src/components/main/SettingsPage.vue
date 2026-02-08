<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../../stores/settings";
import { useUiStore } from "../../stores/ui";
import type { StorageStats } from "../../types";
import ConfirmDialog from "../shared/ConfirmDialog.vue";

const settings = useSettingsStore();
const ui = useUiStore();

const maxLogLines = ref(settings.maxLogLines);
const editorValue = ref(settings.editor ?? "");
const detectedEditor = ref("");
const storageStats = ref<StorageStats | null>(null);
const cleanupDays = ref(30);
const showClearAllDialog = ref(false);
const savingLogLines = ref(false);
const savingEditor = ref(false);

const formattedSize = computed(() => {
  if (!storageStats.value) return "0 B";
  const bytes = storageStats.value.totalSize;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
});

const editorChanged = computed(() => {
  const current = settings.editor ?? "";
  return editorValue.value !== current;
});

async function loadStorageStats() {
  try {
    storageStats.value = await invoke<StorageStats>("get_storage_stats");
  } catch {
    // Ignore
  }
}

async function detectEditor() {
  try {
    detectedEditor.value = await settings.detectSystemEditor();
  } catch {
    detectedEditor.value = "";
  }
}

async function saveLogLines() {
  savingLogLines.value = true;
  try {
    await settings.updateMaxLogLines(maxLogLines.value);
  } finally {
    savingLogLines.value = false;
  }
}

async function saveEditor() {
  savingEditor.value = true;
  try {
    await settings.updateEditor(editorValue.value);
  } finally {
    savingEditor.value = false;
  }
}

function useDetectedEditor() {
  editorValue.value = detectedEditor.value;
}

async function cleanupOldData() {
  try {
    storageStats.value = await invoke<StorageStats>("cleanup_storage", {
      days: cleanupDays.value,
    });
  } catch {
    // Ignore
  }
}

async function clearAllData() {
  try {
    storageStats.value = await invoke<StorageStats>("cleanup_all_storage");
    showClearAllDialog.value = false;
  } catch {
    // Ignore
  }
}

onMounted(() => {
  maxLogLines.value = settings.maxLogLines;
  editorValue.value = settings.editor ?? "";
  loadStorageStats();
  detectEditor();
});
</script>

<template>
  <div class="flex-1 flex flex-col h-full min-h-0">
    <div class="p-4 border-b border-gray-700 flex items-center gap-3">
      <button
        class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
        @click="ui.clearSelection()"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </button>
      <h2 class="text-lg font-semibold text-gray-100">Settings</h2>
    </div>

    <div class="flex-1 overflow-y-auto p-6 space-y-8 max-w-2xl">
      <!-- General Settings -->
      <section>
        <h3 class="text-md font-semibold text-gray-200 mb-4">General</h3>
        <div class="bg-gray-800 rounded-lg p-4 border border-gray-700 space-y-4">
          <div>
            <label class="block text-sm text-gray-400 mb-1">Max Log Lines</label>
            <div class="flex items-center gap-3">
              <input
                v-model.number="maxLogLines"
                type="number"
                min="1000"
                max="100000"
                step="1000"
                class="w-40 px-3 py-2 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500"
              />
              <button
                class="px-4 py-2 text-sm rounded bg-blue-600 text-white hover:bg-blue-500 transition-colors disabled:opacity-50"
                :disabled="savingLogLines || maxLogLines === settings.maxLogLines"
                @click="saveLogLines"
              >
                {{ savingLogLines ? "Saving..." : "Save" }}
              </button>
            </div>
            <p class="text-xs text-gray-500 mt-1">
              Number of log lines to keep in memory per project (1,000 - 100,000).
            </p>
          </div>
        </div>
      </section>

      <!-- Editor Settings -->
      <section>
        <h3 class="text-md font-semibold text-gray-200 mb-4">Editor</h3>
        <div class="bg-gray-800 rounded-lg p-4 border border-gray-700 space-y-4">
          <div>
            <label class="block text-sm text-gray-400 mb-1">Default Editor</label>
            <div class="flex items-center gap-3">
              <input
                v-model="editorValue"
                type="text"
                placeholder="e.g., code, vim, emacs"
                class="flex-1 px-3 py-2 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500"
              />
              <button
                class="px-4 py-2 text-sm rounded bg-blue-600 text-white hover:bg-blue-500 transition-colors disabled:opacity-50"
                :disabled="savingEditor || !editorChanged"
                @click="saveEditor"
              >
                {{ savingEditor ? "Saving..." : "Save" }}
              </button>
            </div>
            <p class="text-xs text-gray-500 mt-1">
              Editor command to use when clicking file paths in logs. Leave empty to auto-detect.
            </p>
            <div v-if="detectedEditor" class="mt-2 flex items-center gap-2">
              <span class="text-xs text-gray-500">Detected:</span>
              <code class="text-xs px-1.5 py-0.5 bg-gray-900 rounded text-green-400">{{ detectedEditor }}</code>
              <button
                v-if="editorValue !== detectedEditor"
                class="text-xs text-blue-400 hover:text-blue-300"
                @click="useDetectedEditor"
              >
                Use this
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- Storage Management -->
      <section>
        <h3 class="text-md font-semibold text-gray-200 mb-4">Storage</h3>
        <div class="bg-gray-800 rounded-lg p-4 border border-gray-700 space-y-4">
          <div v-if="storageStats" class="grid grid-cols-2 gap-4">
            <div>
              <span class="text-xs text-gray-500">Total Size</span>
              <p class="text-lg font-semibold text-gray-200">{{ formattedSize }}</p>
            </div>
            <div>
              <span class="text-xs text-gray-500">Sessions</span>
              <p class="text-lg font-semibold text-gray-200">{{ storageStats.sessionCount }}</p>
            </div>
            <div>
              <span class="text-xs text-gray-500">Log Entries</span>
              <p class="text-lg font-semibold text-gray-200">{{ storageStats.logCount.toLocaleString() }}</p>
            </div>
            <div>
              <span class="text-xs text-gray-500">Metric Points</span>
              <p class="text-lg font-semibold text-gray-200">{{ storageStats.metricCount.toLocaleString() }}</p>
            </div>
          </div>
          <div v-else class="text-sm text-gray-500">Loading storage stats...</div>

          <hr class="border-gray-700" />

          <div>
            <label class="block text-sm text-gray-400 mb-2">Auto-cleanup</label>
            <div class="flex items-center gap-3">
              <span class="text-sm text-gray-400">Delete data older than</span>
              <input
                v-model.number="cleanupDays"
                type="number"
                min="1"
                max="365"
                class="w-20 px-2 py-1 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500"
              />
              <span class="text-sm text-gray-400">days</span>
              <button
                class="px-3 py-1.5 text-sm rounded bg-yellow-600 text-white hover:bg-yellow-500 transition-colors"
                @click="cleanupOldData"
              >
                Clean Up
              </button>
            </div>
          </div>

          <hr class="border-gray-700" />

          <div>
            <button
              class="px-4 py-2 text-sm rounded bg-red-600 text-white hover:bg-red-500 transition-colors"
              @click="showClearAllDialog = true"
            >
              Clear All Data
            </button>
            <p class="text-xs text-gray-500 mt-1">
              Permanently delete all sessions, logs, and metrics.
            </p>
          </div>
        </div>
      </section>
    </div>

    <ConfirmDialog
      :open="showClearAllDialog"
      title="Clear All Data"
      message="Are you sure you want to delete all sessions, logs, and metrics? This cannot be undone."
      confirm-label="Clear All"
      @confirm="clearAllData"
      @cancel="showClearAllDialog = false"
    />
  </div>
</template>
