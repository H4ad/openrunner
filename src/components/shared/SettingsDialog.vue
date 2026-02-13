<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useSettingsStore } from "../../stores/settings";
import { useUpdatesStore } from "../../stores/updates";
import { invoke } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Progress } from "@/components/ui/progress";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DownloadIcon,
  RefreshCwIcon,
  RocketIcon,
  FlaskConicalIcon,
  TerminalIcon,
  CheckCircleIcon,
  XCircleIcon,
  Loader2Icon,
} from "lucide-vue-next";
import MarkdownRenderer from "./MarkdownRenderer.vue";
import ConfirmDialog from "./ConfirmDialog.vue";
import type { StorageStats, CliInstallResult } from "../../types";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const settings = useSettingsStore();
const updates = useUpdatesStore();

const maxLogLines = ref(settings.maxLogLines);
const editorValue = ref(settings.editor ?? "");
const fullscreen = ref(settings.fullscreen ?? false);
const shellValue = ref(settings.shell ?? "");
const minimizeToTray = ref(settings.minimizeToTray);
const autoLaunch = ref(settings.autoLaunch);
const detectedEditor = ref("");
const detectedShell = ref("");
const storageStats = ref<StorageStats | null>(null);
const cleanupDays = ref(30);
const showClearAllDialog = ref(false);
const savingEditor = ref(false);
const savingShell = ref(false);
const loadingStorage = ref(false);

// CLI state
const cliStatus = ref<CliInstallResult | null>(null);
const cliLoading = ref(false);
const cliError = ref<string | null>(null);

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

const shellChanged = computed(() => {
  const current = settings.shell ?? "";
  return shellValue.value !== current;
});

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

async function detectEditor() {
  try {
    detectedEditor.value = await settings.detectSystemEditor();
  } catch {
    detectedEditor.value = "";
  }
}

async function detectShell() {
  try {
    detectedShell.value = await settings.detectSystemShell();
  } catch {
    detectedShell.value = "";
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

async function saveShell() {
  savingShell.value = true;
  try {
    await settings.updateShell(shellValue.value);
  } finally {
    savingShell.value = false;
  }
}

function useDetectedEditor() {
  editorValue.value = detectedEditor.value;
}

function useDetectedShell() {
  shellValue.value = detectedShell.value;
}

async function loadCliStatus() {
  try {
    cliStatus.value = await invoke<CliInstallResult>("cli:get-status");
    cliError.value = null;
  } catch {
    cliStatus.value = null;
  }
}

async function installCli() {
  cliLoading.value = true;
  cliError.value = null;
  try {
    const result = await invoke<CliInstallResult>("cli:install");
    cliStatus.value = result;
    if (!result.success) {
      cliError.value = result.message;
    }
  } catch (error) {
    cliError.value = error instanceof Error ? error.message : String(error);
  } finally {
    cliLoading.value = false;
  }
}

async function uninstallCli() {
  cliLoading.value = true;
  cliError.value = null;
  try {
    const result = await invoke<CliInstallResult>("cli:uninstall");
    cliStatus.value = result;
    if (!result.success) {
      cliError.value = result.message;
    }
  } catch (error) {
    cliError.value = error instanceof Error ? error.message : String(error);
  } finally {
    cliLoading.value = false;
  }
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

// Sample release notes for testing markdown rendering in dev mode
const sampleReleaseNotes = `## What's New

### Features
- Added support for multiple project types
- New dark mode toggle in settings
- Improved process monitoring with real-time stats

### Bug Fixes
- Fixed issue with process logs not scrolling correctly
- Resolved memory leak in stats collector
- Fixed YAML sync not working for nested configurations

### Breaking Changes
- Removed deprecated \`legacyMode\` setting
- Changed default shell to use system detection

### Other
- Updated dependencies to latest versions
- Improved TypeScript types coverage
- Added comprehensive logging for debugging

---
**Full Changelog**: https://github.com/h4ad/openrunner/compare/v1.0.0...v2.0.0`;

function testUpdatePreview() {
  updates.setMockUpdate(sampleReleaseNotes);
}

function handleOpenChange(open: boolean) {
  if (!open) {
    emit("close");
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      maxLogLines.value = settings.maxLogLines;
      editorValue.value = settings.editor ?? "";
      fullscreen.value = settings.fullscreen ?? false;
      shellValue.value = settings.shell ?? "";
      minimizeToTray.value = settings.minimizeToTray;
      autoLaunch.value = settings.autoLaunch;
      loadStorageStats();
      detectEditor();
      detectShell();
      loadCliStatus();
    }
  },
);

// Auto-save settings when values change
watch(maxLogLines, async (value) => {
  if (props.open && value !== settings.maxLogLines) {
    await settings.updateMaxLogLines(value);
  }
});

watch(fullscreen, async (value) => {
  if (props.open && value !== settings.fullscreen) {
    await settings.updateFullscreen(value);
  }
});

watch(minimizeToTray, async (value) => {
  if (props.open && value !== settings.minimizeToTray) {
    await settings.updateMinimizeToTray(value);
  }
});

watch(autoLaunch, async (value) => {
  if (props.open && value !== settings.autoLaunch) {
    await settings.updateAutoLaunch(value);
  }
});
</script>

<template>
  <Dialog :open="props.open" @update:open="handleOpenChange">
    <DialogContent class="sm:max-w-2xl max-h-[85vh] overflow-y-auto">
      <DialogHeader>
        <DialogTitle>Settings</DialogTitle>
      </DialogHeader>

      <div class="space-y-6 py-4">
        <!-- General Settings -->
        <section>
          <h3 class="text-sm font-semibold text-foreground mb-3">General</h3>
          <Card>
            <CardContent class="px-4 py-4 space-y-4">
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
                </div>
                <p class="text-xs text-muted-foreground">
                  Number of log lines to keep in memory per project (1,000 - 100,000).
                </p>
              </div>

              <Separator />

              <div class="flex items-center justify-between space-y-0">
                <div class="space-y-0.5">
                  <Label for="fullscreen">Fullscreen</Label>
                  <p class="text-xs text-muted-foreground">
                    Start app in fullscreen mode (toggle with F11).
                  </p>
                </div>
                <Switch
                  id="fullscreen"
                  :model-value="fullscreen"
                  @update:model-value="fullscreen = $event"
                />
              </div>

              <Separator />

              <div class="flex items-center justify-between space-y-0">
                <div class="space-y-0.5">
                  <Label for="minimize-to-tray">Minimize to Tray</Label>
                  <p class="text-xs text-muted-foreground">
                    Keep app in system tray when closing. Right-click tray icon to quit.
                  </p>
                </div>
                <Switch
                  id="minimize-to-tray"
                  :model-value="minimizeToTray"
                  @update:model-value="minimizeToTray = $event"
                />
              </div>

              <Separator />

              <div class="flex items-center justify-between space-y-0">
                <div class="space-y-0.5">
                  <Label for="auto-launch">Start with System</Label>
                  <p class="text-xs text-muted-foreground">
                    Automatically start OpenRunner when you log in.
                  </p>
                </div>
                <Switch
                  id="auto-launch"
                  :model-value="autoLaunch"
                  @update:model-value="autoLaunch = $event"
                />
              </div>

            </CardContent>
          </Card>
        </section>

        <!-- Editor Settings -->
        <section>
          <h3 class="text-sm font-semibold text-foreground mb-3">Editor</h3>
          <Card>
            <CardContent class="px-4 py-4 space-y-4">
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

        <!-- Shell Settings -->
        <section>
          <h3 class="text-sm font-semibold text-foreground mb-3">Shell</h3>
          <Card>
            <CardContent class="px-4 py-4 space-y-4">
              <div class="space-y-2">
                <Label for="default-shell">Default Shell</Label>
                <div class="flex items-center gap-3">
                  <Input
                    id="default-shell"
                    v-model="shellValue"
                    placeholder="e.g., /bin/bash, /bin/zsh, cmd.exe"
                    class="flex-1"
                  />
                  <Button
                    :disabled="savingShell || !shellChanged"
                    @click="saveShell"
                  >
                    {{ savingShell ? "Saving..." : "Save" }}
                  </Button>
                </div>
                <p class="text-xs text-muted-foreground">
                  Shell to use when running project commands. Leave empty to auto-detect from environment.
                </p>
                <div v-if="detectedShell" class="flex items-center gap-2">
                  <span class="text-xs text-muted-foreground">Detected:</span>
                  <code class="text-xs px-1.5 py-0.5 bg-muted rounded text-green-400">{{ detectedShell }}</code>
                  <Button
                    v-if="shellValue !== detectedShell"
                    variant="link"
                    size="sm"
                    class="text-xs h-auto p-0"
                    @click="useDetectedShell"
                  >
                    Use this
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        </section>

        <!-- CLI Section -->
        <section>
          <h3 class="text-sm font-semibold text-foreground mb-3">Command Line</h3>
          <Card>
            <CardContent class="px-4 py-4 space-y-4">
              <div class="flex items-center justify-between">
                <div class="space-y-1">
                  <div class="flex items-center gap-2">
                    <TerminalIcon class="h-4 w-4 text-muted-foreground" />
                    <Label>CLI Command</Label>
                  </div>
                  <p class="text-xs text-muted-foreground">
                    Install the <code class="px-1 py-0.5 bg-muted rounded">openrunner</code> command to create groups from your terminal.
                  </p>
                </div>
                <div class="flex items-center gap-3">
                  <div v-if="cliStatus" class="flex items-center gap-1.5">
                    <CheckCircleIcon v-if="cliStatus.installed" class="h-4 w-4 text-green-500" />
                    <XCircleIcon v-else class="h-4 w-4 text-muted-foreground" />
                    <span class="text-xs" :class="cliStatus.installed ? 'text-green-500' : 'text-muted-foreground'">
                      {{ cliStatus.installed ? "Installed" : "Not installed" }}
                    </span>
                  </div>
                  <Button
                    v-if="cliStatus?.installed"
                    variant="destructive"
                    size="sm"
                    :disabled="cliLoading"
                    @click="uninstallCli"
                  >
                    <Loader2Icon v-if="cliLoading" class="h-4 w-4 mr-2 animate-spin" />
                    Uninstall
                  </Button>
                  <Button
                    v-else
                    size="sm"
                    :disabled="cliLoading"
                    @click="installCli"
                  >
                    <Loader2Icon v-if="cliLoading" class="h-4 w-4 mr-2 animate-spin" />
                    Install CLI
                  </Button>
                </div>
              </div>

              <div v-if="cliStatus?.installed && cliStatus.path" class="text-xs text-muted-foreground">
                Installed at: <code class="px-1 py-0.5 bg-muted rounded">{{ cliStatus.path }}</code>
              </div>

              <div v-if="cliError" class="text-xs text-red-400 bg-red-950/50 p-3 rounded-md whitespace-pre-wrap">
                {{ cliError }}
              </div>

              <Separator />

              <div class="space-y-2">
                <p class="text-xs font-medium text-foreground">Usage:</p>
                <div class="bg-muted p-3 rounded-md space-y-1">
                  <code class="block text-xs text-green-400">openrunner new .</code>
                  <span class="text-xs text-muted-foreground">Auto-detect projects in current directory</span>
                </div>
                <div class="bg-muted p-3 rounded-md space-y-1">
                  <code class="block text-xs text-green-400">openrunner new ~/myproject --name "My Project"</code>
                  <span class="text-xs text-muted-foreground">Create group with a specific name</span>
                </div>
              </div>
            </CardContent>
          </Card>
        </section>

        <!-- Updates Section -->
        <section>
          <h3 class="text-sm font-semibold text-foreground mb-3">Updates</h3>
          <Card>
            <CardContent class="px-4 py-4 space-y-4">
              <!-- Current Version -->
              <div class="flex items-center justify-between">
                <div>
                  <span class="text-xs text-muted-foreground">Current Version</span>
                  <p class="text-lg font-semibold text-foreground">
                    v{{ updates.currentVersion || "..." }}
                  </p>
                </div>
                <div class="flex gap-2">
                  <Button
                    v-if="updates.isDevMode"
                    variant="outline"
                    :disabled="updates.checking || updates.downloading"
                    @click="testUpdatePreview()"
                  >
                    <FlaskConicalIcon class="h-4 w-4 mr-2" />
                    Test Preview
                  </Button>
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
              </div>

              <!-- Update Available -->
              <template v-if="updates.available && !updates.downloaded">
                <Separator />
                <div class="space-y-3">
                  <div class="flex items-center justify-between">
                    <div>
                      <p class="text-sm font-medium text-foreground">
                        Version {{ updates.updateVersion }} available
                        <span
                          v-if="updates.isDevMode && updates.updateVersion === '99.0.0-preview'"
                          class="ml-2 text-xs text-amber-400"
                        >
                          (Preview Mode)
                        </span>
                      </p>
                      <p v-if="updates.releaseDate" class="text-xs text-muted-foreground">
                        Released {{ new Date(updates.releaseDate).toLocaleDateString() }}
                      </p>
                    </div>
                    <div class="flex gap-2">
                      <Button
                        v-if="updates.isDevMode && updates.updateVersion === '99.0.0-preview'"
                        variant="ghost"
                        @click="updates.clearMockUpdate()"
                      >
                        Clear Preview
                      </Button>
                      <Button
                        v-if="!updates.downloading && updates.updateVersion !== '99.0.0-preview'"
                        @click="updates.downloadUpdate()"
                      >
                        <DownloadIcon class="h-4 w-4 mr-2" />
                        {{ updates.autoUpdateSupported ? "Download" : "View Release" }}
                      </Button>
                    </div>
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
                    class="bg-muted p-3 rounded-md max-h-64 overflow-y-auto release-notes-container"
                  >
                    <p class="text-xs font-medium mb-2 text-foreground">Release Notes:</p>
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
          <h3 class="text-sm font-semibold text-foreground mb-3">Storage</h3>
          <Card>
            <CardContent class="px-4 py-4 space-y-4">
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

      <div class="flex justify-end pt-4 border-t">
        <Button variant="secondary" @click="emit('close')">
          Close
        </Button>
      </div>
    </DialogContent>
  </Dialog>

  <ConfirmDialog
    :open="showClearAllDialog"
    title="Clear All Data"
    message="Are you sure you want to delete all sessions, logs, and metrics? This cannot be undone."
    confirm-label="Clear All"
    @confirm="clearAllData"
    @cancel="showClearAllDialog = false"
  />
</template>

<style scoped>
.release-notes-container {
  scrollbar-width: thin;
  scrollbar-color: hsl(var(--muted-foreground) / 0.3) transparent;
}

.release-notes-container::-webkit-scrollbar {
  width: 6px;
}

.release-notes-container::-webkit-scrollbar-track {
  background: transparent;
}

.release-notes-container::-webkit-scrollbar-thumb {
  background-color: hsl(var(--muted-foreground) / 0.3);
  border-radius: 3px;
}

.release-notes-container::-webkit-scrollbar-thumb:hover {
  background-color: hsl(var(--muted-foreground) / 0.5);
}
</style>
