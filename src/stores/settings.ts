import { defineStore } from "pinia";
import { ref } from "vue";
import type { AppSettings } from "../types";
import * as db from "../services/database";

export const useSettingsStore = defineStore("settings", () => {
  const maxLogLines = ref(10_000);
  const editor = ref<string | null>(null);

  async function load() {
    const settings = await db.getSettings();
    maxLogLines.value = settings.maxLogLines;
    editor.value = settings.editor;
  }

  async function updateMaxLogLines(value: number) {
    const settings: AppSettings = {
      maxLogLines: value,
      editor: editor.value,
    };
    await db.updateSettings(settings);
    maxLogLines.value = value;
  }

  async function updateEditor(value: string) {
    const settings: AppSettings = {
      maxLogLines: maxLogLines.value,
      editor: value,
    };
    await db.updateSettings(settings);
    editor.value = value;
  }

  async function detectSystemEditor(): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
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
