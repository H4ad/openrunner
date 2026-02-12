<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch, provide } from "vue";
import { listen, type UnlistenFn } from "@/lib/api";
import { useConfigStore } from "./stores/config";
import { useProcessesStore } from "./stores/processes";
import { useLogsStore } from "./stores/logs";
import { useSettingsStore } from "./stores/settings";
import { useUpdatesStore } from "./stores/updates";
import { useUiStore } from "./stores/ui";
import Sidebar from "./components/sidebar/Sidebar.vue";
import MainPanel from "./components/main/MainPanel.vue";
import SettingsDialog from "./components/shared/SettingsDialog.vue";
import { Toaster } from "@/components/ui/sonner";
import { toast } from "vue-sonner";

const config = useConfigStore();
const processes = useProcessesStore();
const logsStore = useLogsStore();
const settingsStore = useSettingsStore();
const updatesStore = useUpdatesStore();
const ui = useUiStore();

const showSettingsDialog = ref(false);

provide("showSettingsDialog", () => {
  showSettingsDialog.value = true;
});

const sidebarWidth = ref(256);
const isResizing = ref(false);
const isShuttingDown = ref(false);
const showSidebar = computed(() => ui.viewMode !== "home");

let unlistenAppClosing: UnlistenFn | null = null;

function startResize(e: MouseEvent) {
  e.preventDefault();
  isResizing.value = true;

  const onMouseMove = (e: MouseEvent) => {
    const newWidth = Math.max(180, Math.min(480, e.clientX));
    sidebarWidth.value = newWidth;
  };

  const onMouseUp = () => {
    isResizing.value = false;
    document.removeEventListener("mousemove", onMouseMove);
    document.removeEventListener("mouseup", onMouseUp);
  };

  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", onMouseUp);
}

onMounted(async () => {
  await settingsStore.load();
  await config.init();
  await processes.init();
  await logsStore.init();
  await updatesStore.initialize();

  // Listen for app closing event to show shutdown UI
  unlistenAppClosing = await listen("app-closing", () => {
    isShuttingDown.value = true;
  });
});

  // Watch for update availability and show toast
  watch(
    () => updatesStore.available,
    (available) => {
      if (available && updatesStore.updateVersion) {
        toast.info(`Update ${updatesStore.updateVersion} available`, {
          description: "Go to Settings to download and install.",
          action: {
            label: "View",
            onClick: () => showSettingsDialog.value = true,
          },
          duration: 10000,
        });
      }
    }
  );

// Watch for update downloaded and show toast
watch(
  () => updatesStore.downloaded,
  (downloaded) => {
    if (downloaded && updatesStore.updateVersion) {
      toast.success(`Update ${updatesStore.updateVersion} ready`, {
        description: "Restart to apply the update.",
        action: {
          label: "Restart",
          onClick: () => updatesStore.installUpdate(),
        },
        duration: 15000,
      });
    }
  }
);

onUnmounted(() => {
  unlistenAppClosing?.();
});
</script>

<template>
  <div
    class="h-screen w-screen bg-gray-900 text-gray-100 flex overflow-hidden relative"
    :class="isResizing ? 'select-none' : ''"
  >
    <template v-if="showSidebar">
      <Sidebar :style="{ width: sidebarWidth + 'px', minWidth: sidebarWidth + 'px' }" />
      <div
        class="w-1 cursor-col-resize bg-gray-700 hover:bg-blue-500 transition-colors flex-shrink-0"
        :class="isResizing ? 'bg-blue-500' : ''"
        @mousedown="startResize"
      />
      <MainPanel />
    </template>
    <MainPanel v-else />

    <!-- Shutdown overlay -->
    <div
      v-if="isShuttingDown"
      class="absolute inset-0 bg-gray-900/90 flex items-center justify-center z-50"
    >
      <div class="text-center">
        <div class="animate-spin w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-4"></div>
        <p class="text-lg text-gray-200">Stopping processes...</p>
        <p class="text-sm text-gray-400 mt-2">Please wait while processes are gracefully terminated</p>
      </div>
    </div>

    <!-- Toast notifications -->
    <Toaster position="bottom-right" :visible-toasts="3" />

    <!-- Settings Dialog -->
    <SettingsDialog
      :open="showSettingsDialog"
      @close="showSettingsDialog = false"
    />
  </div>
</template>
