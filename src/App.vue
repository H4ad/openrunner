<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { useConfigStore } from "./stores/config";
import { useProcessesStore } from "./stores/processes";
import { useLogsStore } from "./stores/logs";
import { useSettingsStore } from "./stores/settings";
import { useUiStore } from "./stores/ui";
import Sidebar from "./components/sidebar/Sidebar.vue";
import MainPanel from "./components/main/MainPanel.vue";

const config = useConfigStore();
const processes = useProcessesStore();
const logsStore = useLogsStore();
const settingsStore = useSettingsStore();
const ui = useUiStore();

const sidebarWidth = ref(256);
const isResizing = ref(false);
const isShuttingDown = ref(false);
const showSidebar = computed(() => ui.viewMode !== "home");

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

  // Listen for app closing event to show shutdown UI
  listen("app-closing", () => {
    isShuttingDown.value = true;
  });
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
  </div>
</template>
