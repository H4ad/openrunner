import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@/lib/api";
import type { AppSettings } from "../types";

export const useSettingsStore = defineStore("settings", () => {
  const maxLogLines = ref(10_000);
  const editor = ref<string | null>(null);
  const fullscreen = ref<boolean | null>(null);

  async function load() {
    const settings = await invoke<AppSettings>("get_settings");
    maxLogLines.value = settings.maxLogLines;
    editor.value = settings.editor;
    fullscreen.value = settings.fullscreen;
  }

  async function updateMaxLogLines(value: number) {
    const settings: AppSettings = {
      maxLogLines: value,
      editor: editor.value,
      fullscreen: fullscreen.value,
    };
    await invoke("update_settings", { settings });
    maxLogLines.value = value;
  }

  async function updateEditor(value: string) {
    const settings: AppSettings = {
      maxLogLines: maxLogLines.value,
      editor: value,
      fullscreen: fullscreen.value,
    };
    await invoke("update_settings", { settings });
    editor.value = value;
  }

  async function updateFullscreen(value: boolean) {
    const settings: AppSettings = {
      maxLogLines: maxLogLines.value,
      editor: editor.value,
      fullscreen: value,
    };
    await invoke("update_settings", { settings });
    fullscreen.value = value;
  }

  async function toggleFullscreen() {
    const isFullscreen = await invoke<boolean>("window:toggle-fullscreen");
    fullscreen.value = isFullscreen;
  }

  async function detectSystemEditor(): Promise<string> {
    return await invoke<string>("detect_system_editor");
  }

  return {
    maxLogLines,
    editor,
    fullscreen,
    load,
    updateMaxLogLines,
    updateEditor,
    updateFullscreen,
    toggleFullscreen,
    detectSystemEditor,
  };
});
