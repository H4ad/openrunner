<script setup lang="ts">
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Progress } from "@/components/ui/progress";
import { ArrowLeftIcon } from "@radix-icons/vue";
import { DownloadIcon, RefreshCwIcon, RocketIcon } from "lucide-vue-next";
import { invoke } from "@/lib/api";
import { type as getOsType } from "@/lib/os";
import { computed, onMounted, ref } from "vue";
import { useSettingsStore } from "../../stores/settings";
import { useUpdatesStore } from "../../stores/updates";
import { useUiStore } from "../../stores/ui";
import type { StorageStats } from "../../types";
import ConfirmDialog from "../shared/ConfirmDialog.vue";
import MarkdownRenderer from "../shared/MarkdownRenderer.vue";

const settings = useSettingsStore();
const updates = useUpdatesStore();
const ui = useUiStore();

const maxLogLines = ref(settings.maxLogLines);
const editorValue = ref(settings.editor ?? "");
const detectedEditor = ref("");
const storageStats = ref<StorageStats | null>(null);
const cleanupDays = ref(30);
const showClearAllDialog = ref(false);
const savingLogLines = ref(false);
const savingEditor = ref(false);
const savingLinuxGpu = ref(false);
const osType = ref<string | null>(null);
const isLinux = computed(() => osType.value === "linux");

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

async function updateLinuxGpuOptimization(value: boolean) {
  savingLinuxGpu.value = true;
  try {
    await settings.updateLinuxGpuOptimization(value);
  } finally {
    savingLinuxGpu.value = false;
  }
}

onMounted(async () => {
  maxLogLines.value = settings.maxLogLines;
  editorValue.value = settings.editor ?? "";
  loadStorageStats();
  detectEditor();
  osType.value = await getOsType();
});
</script>

<template>
  <div class="flex-1 flex flex-col h-full min-h-0">
    <div class="p-4 border-b border-border flex items-center gap-3">
      <Button variant="ghost" size="icon" class="h-8 w-8" @click="ui.clearSelection()">
        <ArrowLeftIcon class="h-4 w-4" />
      </Button>
      <h2 class="text-lg font-semibold text-foreground">Settings</h2>
    </div>

    <div class="flex-1 overflow-y-auto p-6 space-y-8 max-w-2xl">
      <!-- General Settings -->
      <section>
        <h3 class="text-md font-semibold text-foreground mb-4">General</h3>
        <Card>
          <CardContent class="px-4 space-y-4">
            <div class="space-y-2">
              <Label for="max-log-lines">Max Log Lines</Label>
              <div class="flex items-center gap-3">
                <Input
                  id="max-log-lines"
                  v-model.number="maxLogLines"
                  type="number"
                  min="1000"
                  max="100000"
                  step="1000"
                  class="w-40"
                />
                <Button
                  :disabled="savingLogLines || maxLogLines === settings.maxLogLines"
                  @click="saveLogLines"
                >
                  {{ savingLogLines ? "Saving..." : "Save" }}
                </Button>
              </div>
              <p class="text-xs text-muted-foreground">
                Number of log lines to keep in memory per project (1,000 - 100,000).
              </p>
            </div>
          </CardContent>
        </Card>
      </section>

      <!-- Linux GPU Optimization (Linux only) -->
      <section v-if="isLinux">
        <h3 class="text-md font-semibold text-foreground mb-4">Platform</h3>
        <Card>
          <CardContent class="px-4 space-y-4">
            <div class="flex items-center justify-between">
              <div class="space-y-0.5">
                <Label for="linux-gpu-opt">GPU Optimization</Label>
                <p class="text-xs text-muted-foreground">
                  Disable hardware acceleration to fix scroll lag and rendering issues.
                  Requires restart to take effect.
                </p>
              </div>
              <Switch
                id="linux-gpu-opt"
                :model-value="settings.linuxGpuOptimization ?? true"
                @update:model-value="updateLinuxGpuOptimization"
              />
            </div>
          </CardContent>
        </Card>
      </section>

      <!-- Editor Settings -->
      <section>
        <h3 class="text-md font-semibold text-foreground mb-4">Editor</h3>
        <Card>
          <CardContent class="px-4 space-y-4">
            <div class="space-y-2">
              <Label for="default-editor">Default Editor</Label>
              <div class="flex items-center gap-3">
                <Input
                  id="default-editor"
                  v-model="editorValue"
                  placeholder="e.g., code, vim, emacs"
                  class="flex-1"
                />
                <Button
                  :disabled="savingEditor || !editorChanged"
                  @click="saveEditor"
                >
                  {{ savingEditor ? "Saving..." : "Save" }}
                </Button>
              </div>
              <p class="text-xs text-muted-foreground">
                Editor command to use when clicking file paths in logs. Leave empty to auto-detect.
              </p>
              <div v-if="detectedEditor" class="flex items-center gap-2">
                <span class="text-xs text-muted-foreground">Detected:</span>
                <code class="text-xs px-1.5 py-0.5 bg-muted rounded text-green-400">{{ detectedEditor }}</code>
                <Button
                  v-if="editorValue !== detectedEditor"
                  variant="link"
                  size="sm"
                  class="text-xs h-auto p-0"
                  @click="useDetectedEditor"
                >
                  Use this
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </section>

      <!-- Updates Section -->
      <section>
        <h3 class="text-md font-semibold text-foreground mb-4">Updates</h3>
        <Card>
          <CardContent class="px-4 space-y-4">
            <!-- Current Version -->
            <div class="flex items-center justify-between">
              <div>
                <span class="text-xs text-muted-foreground">Current Version</span>
                <p class="text-lg font-semibold text-foreground">
                  v{{ updates.currentVersion || "..." }}
                </p>
              </div>
              <Button
                variant="secondary"
                :disabled="updates.checking || updates.downloading"
                @click="updates.checkForUpdates()"
              >
                <RefreshCwIcon
                  class="h-4 w-4 mr-2"
                  :class="{ 'animate-spin': updates.checking }"
                />
                {{ updates.checking ? "Checking..." : "Check for Updates" }}
              </Button>
            </div>

            <!-- Update Available -->
            <template v-if="updates.available && !updates.downloaded">
              <Separator />
              <div class="space-y-3">
                <div class="flex items-center justify-between">
                  <div>
                    <p class="text-sm font-medium text-foreground">
                      Version {{ updates.updateVersion }} available
                    </p>
                    <p v-if="updates.releaseDate" class="text-xs text-muted-foreground">
                      Released {{ new Date(updates.releaseDate).toLocaleDateString() }}
                    </p>
                  </div>
                  <Button
                    v-if="!updates.downloading"
                    @click="updates.downloadUpdate()"
                  >
                    <DownloadIcon class="h-4 w-4 mr-2" />
                    {{ updates.autoUpdateSupported ? "Download" : "View Release" }}
                  </Button>
                </div>

                <!-- Download Progress -->
                <div v-if="updates.downloading" class="space-y-2">
                  <div class="flex items-center justify-between text-xs text-muted-foreground">
                    <span>Downloading...</span>
                    <span>{{ Math.round(updates.progress) }}%</span>
                  </div>
                  <Progress :model-value="updates.progress" class="h-2" />
                </div>

                <!-- Release Notes Preview -->
                <div
                  v-if="updates.releaseNotes"
                  class="text-xs text-muted-foreground bg-muted p-3 rounded-md max-h-32 overflow-y-auto"
                >
                  <p class="font-medium mb-2 text-foreground">Release Notes:</p>
                  <MarkdownRenderer :content="updates.releaseNotes" />
                </div>
              </div>
            </template>

            <!-- Update Downloaded -->
            <template v-else-if="updates.downloaded">
              <Separator />
              <div class="flex items-center justify-between">
                <div>
                  <p class="text-sm font-medium text-green-400">
                    Version {{ updates.updateVersion }} ready to install
                  </p>
                  <p class="text-xs text-muted-foreground">
                    The app will restart to apply the update.
                  </p>
                </div>
                <Button @click="updates.installUpdate()">
                  <RocketIcon class="h-4 w-4 mr-2" />
                  Restart and Update
                </Button>
              </div>
            </template>

            <!-- No Update Available -->
            <template v-else-if="!updates.checking && !updates.available">
              <p class="text-xs text-muted-foreground">
                You're running the latest version.
              </p>
            </template>

            <!-- Error -->
            <template v-if="updates.error">
              <Separator />
              <div class="text-xs text-red-400 bg-red-950/50 p-3 rounded-md">
                <p class="font-medium">Update Error:</p>
                <p>{{ updates.error }}</p>
                <Button
                  variant="ghost"
                  size="sm"
                  class="mt-2 h-6 text-xs"
                  @click="updates.clearError()"
                >
                  Dismiss
                </Button>
              </div>
            </template>

            <!-- macOS Notice -->
            <template v-if="!updates.autoUpdateSupported && updates.available">
              <p class="text-xs text-amber-400">
                Note: Auto-updates are not available on macOS without code signing.
                Clicking "View Release" will open the download page.
              </p>
            </template>
          </CardContent>
        </Card>
      </section>

      <!-- Storage Management -->
      <section>
        <h3 class="text-md font-semibold text-foreground mb-4">Storage</h3>
        <Card>
          <CardContent class="px-4 space-y-4">
            <div v-if="storageStats" class="grid grid-cols-2 gap-4">
              <div>
                <span class="text-xs text-muted-foreground">Total Size</span>
                <p class="text-lg font-semibold text-foreground">{{ formattedSize }}</p>
              </div>
              <div>
                <span class="text-xs text-muted-foreground">Sessions</span>
                <p class="text-lg font-semibold text-foreground">{{ storageStats.sessionCount }}</p>
              </div>
              <div>
                <span class="text-xs text-muted-foreground">Log Entries</span>
                <p class="text-lg font-semibold text-foreground">{{ storageStats.logCount.toLocaleString() }}</p>
              </div>
              <div>
                <span class="text-xs text-muted-foreground">Metric Points</span>
                <p class="text-lg font-semibold text-foreground">{{ storageStats.metricCount.toLocaleString() }}</p>
              </div>
            </div>
            <div v-else class="text-sm text-muted-foreground">Loading storage stats...</div>

            <Separator />

            <div>
              <Label class="block text-sm mb-2">Auto-cleanup</Label>
              <div class="flex items-center gap-3">
                <span class="text-sm text-muted-foreground">Delete data older than</span>
                <Input
                  v-model.number="cleanupDays"
                  type="number"
                  min="1"
                  max="365"
                  class="w-20"
                />
                <span class="text-sm text-muted-foreground">days</span>
                <Button variant="secondary" @click="cleanupOldData">
                  Clean Up
                </Button>
              </div>
            </div>

            <Separator />

            <div>
              <Button variant="destructive" @click="showClearAllDialog = true">
                Clear All Data
              </Button>
              <p class="text-xs text-muted-foreground mt-1">
                Permanently delete all sessions, logs, and metrics.
              </p>
            </div>
          </CardContent>
        </Card>
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
