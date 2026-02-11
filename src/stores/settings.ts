import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";

export const useSettingsStore = defineStore("settings", () => {
  const maxLogLines = ref(10_000);
  const editor = ref<string | null>(null);
  const linuxGpuOptimization = ref<boolean | null>(null);

  async function load() {
    const settings = await invoke<AppSettings>("get_settings");
    maxLogLines.value = settings.maxLogLines;
    editor.value = settings.editor;
    linuxGpuOptimization.value = settings.linuxGpuOptimization;
  }

  async function updateMaxLogLines(value: number) {
    const settings: AppSettings = {
      maxLogLines: value,
      editor: editor.value,
      linuxGpuOptimization: linuxGpuOptimization.value,
    };
    await invoke("update_settings", { settings });
    maxLogLines.value = value;
  }

  async function updateEditor(value: string) {
    const settings: AppSettings = {
      maxLogLines: maxLogLines.value,
      editor: value,
      linuxGpuOptimization: linuxGpuOptimization.value,
    };
    await invoke("update_settings", { settings });
    editor.value = value;
  }

  async function updateLinuxGpuOptimization(value: boolean) {
    const settings: AppSettings = {
      maxLogLines: maxLogLines.value,
      editor: editor.value,
      linuxGpuOptimization: value,
    };
    await invoke("update_settings", { settings });
    linuxGpuOptimization.value = value;
  }

  async function detectSystemEditor(): Promise<string> {
    return await invoke<string>("detect_system_editor");
  }

  return {
    maxLogLines,
    editor,
    linuxGpuOptimization,
    load,
    updateMaxLogLines,
    updateEditor,
    updateLinuxGpuOptimization,
    detectSystemEditor,
  };
});
