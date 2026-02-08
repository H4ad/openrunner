import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";

export const useSettingsStore = defineStore("settings", () => {
  const maxLogLines = ref(10_000);
  const editor = ref<string | null>(null);

  async function load() {
    const settings = await invoke<AppSettings>("get_settings");
    maxLogLines.value = settings.maxLogLines;
    editor.value = settings.editor;
  }

  async function updateMaxLogLines(value: number) {
    const settings = await invoke<AppSettings>("update_settings", {
      maxLogLines: value,
    });
    maxLogLines.value = settings.maxLogLines;
    editor.value = settings.editor;
  }

  async function updateEditor(value: string) {
    const settings = await invoke<AppSettings>("update_settings", {
      editor: value,
    });
    maxLogLines.value = settings.maxLogLines;
    editor.value = settings.editor;
  }

  async function detectSystemEditor(): Promise<string> {
    return await invoke<string>("detect_system_editor");
  }

  return {
    maxLogLines,
    editor,
    load,
    updateMaxLogLines,
    updateEditor,
    detectSystemEditor,
  };
});
