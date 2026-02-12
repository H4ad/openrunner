import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@/lib/api";
import type { AppSettings } from "../types";

export const useSettingsStore = defineStore("settings", () => {
  const maxLogLines = ref(10_000);
  const editor = ref<string | null>(null);
  const fullscreen = ref<boolean | null>(null);
  const shell = ref<string | null>(null);
  const minimizeToTray = ref(false);

  async function load() {
    const settings = await invoke<AppSettings>("get_settings");
    maxLogLines.value = settings.maxLogLines;
    editor.value = settings.editor;
    fullscreen.value = settings.fullscreen;
    shell.value = settings.shell;
    minimizeToTray.value = settings.minimizeToTray;
  }

  async function updateMaxLogLines(value: number) {
    const settings: AppSettings = {
      maxLogLines: value,
      editor: editor.value,
      fullscreen: fullscreen.value,
      shell: shell.value,
      minimizeToTray: minimizeToTray.value,
    };
    await invoke("update_settings", { settings });
    maxLogLines.value = value;
  }

  async function updateEditor(value: string) {
    const settings: AppSettings = {
      maxLogLines: maxLogLines.value,
      editor: value,
      fullscreen: fullscreen.value,
      shell: shell.value,
      minimizeToTray: minimizeToTray.value,
    };
    await invoke("update_settings", { settings });
    editor.value = value;
  }

  async function updateFullscreen(value: boolean) {
    const settings: AppSettings = {
      maxLogLines: maxLogLines.value,
      editor: editor.value,
      fullscreen: value,
      shell: shell.value,
      minimizeToTray: minimizeToTray.value,
    };
    await invoke("update_settings", { settings });
    fullscreen.value = value;
  }

  async function updateShell(value: string) {
    const settings: AppSettings = {
      maxLogLines: maxLogLines.value,
      editor: editor.value,
      fullscreen: fullscreen.value,
      shell: value,
      minimizeToTray: minimizeToTray.value,
    };
    await invoke("update_settings", { settings });
    shell.value = value;
  }

  async function updateMinimizeToTray(value: boolean) {
    const settings: AppSettings = {
      maxLogLines: maxLogLines.value,
      editor: editor.value,
      fullscreen: fullscreen.value,
      shell: shell.value,
      minimizeToTray: value,
    };
    await invoke("update_settings", { settings });
    minimizeToTray.value = value;
  }

  async function toggleFullscreen() {
    const isFullscreen = await invoke<boolean>("window:toggle-fullscreen");
    fullscreen.value = isFullscreen;
  }

  async function detectSystemEditor(): Promise<string> {
    return await invoke<string>("detect_system_editor");
  }

  async function detectSystemShell(): Promise<string> {
    return await invoke<string>("detect_system_shell");
  }

  return {
    maxLogLines,
    editor,
    fullscreen,
    shell,
    minimizeToTray,
    load,
    updateMaxLogLines,
    updateEditor,
    updateFullscreen,
    updateShell,
    updateMinimizeToTray,
    toggleFullscreen,
    detectSystemEditor,
    detectSystemShell,
  };
});
