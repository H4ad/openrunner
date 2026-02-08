<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { SearchAddon } from "@xterm/addon-search";
import { open } from "@tauri-apps/plugin-shell";
import "@xterm/xterm/css/xterm.css";
import { useLogsStore } from "../../stores/logs";
import { useSettingsStore } from "../../stores/settings";
import type { LogMessage } from "../../types";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps<{
  projectId: string;
  groupId?: string;
}>();

const emit = defineEmits<{
  clear: [];
}>();

const terminalRef = ref<HTMLDivElement>();
const logs = useLogsStore();
const settingsStore = useSettingsStore();

const showSearch = ref(false);
const searchQuery = ref("");
const searchInputRef = ref<HTMLInputElement>();

let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let searchAddon: SearchAddon | null = null;
let unlisten: UnlistenFn | null = null;
let pendingWrites: string[] = [];
let rafId: number | null = null;

function flushWrites() {
  if (!terminal || pendingWrites.length === 0) return;
  const batch = pendingWrites.join("");
  pendingWrites = [];
  terminal.write(batch);
  rafId = null;
}

function scheduleWrite(data: string) {
  pendingWrites.push(data);
  if (rafId === null) {
    rafId = requestAnimationFrame(flushWrites);
  }
}

async function writeExistingLogs() {
  if (!terminal) return;

  // First try in-memory logs
  const existing = logs.getProjectLogs(props.projectId);
  if (existing.length > 0) {
    const batch = existing.map((m) => m.data).join("");
    terminal.write(batch);
    return;
  }

  // Fallback: load from temp file on disk
  try {
    const data = await invoke<string>("read_project_logs", {
      projectId: props.projectId,
    });
    if (data) {
      terminal.write(data);
    }
  } catch {
    // No saved logs
  }
}

function toggleSearch() {
  showSearch.value = !showSearch.value;
  if (showSearch.value) {
    setTimeout(() => searchInputRef.value?.focus(), 0);
  } else {
    searchQuery.value = "";
    searchAddon?.clearDecorations();
  }
}

function findNext() {
  if (searchAddon && searchQuery.value) {
    searchAddon.findNext(searchQuery.value);
  }
}

function findPrevious() {
  if (searchAddon && searchQuery.value) {
    searchAddon.findPrevious(searchQuery.value);
  }
}

function onSearchInput() {
  if (searchAddon && searchQuery.value) {
    searchAddon.findNext(searchQuery.value);
  } else {
    searchAddon?.clearDecorations();
  }
}

function onSearchKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    e.preventDefault();
    if (e.shiftKey) {
      findPrevious();
    } else {
      findNext();
    }
  } else if (e.key === "Escape") {
    toggleSearch();
  }
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === "f") {
    e.preventDefault();
    toggleSearch();
  }
}

async function openFilePath(filePath: string, line?: number, column?: number) {
  try {
    let workingDir = "";
    if (props.groupId) {
      workingDir = await invoke<string>("resolve_project_working_dir", {
        groupId: props.groupId,
        projectId: props.projectId,
      });
    }
    await invoke("open_file_in_editor", {
      filePath,
      line: line ?? null,
      column: column ?? null,
      workingDir,
    });
  } catch {
    // File not found or editor not available
  }
}

async function setupTerminal() {
  if (!terminalRef.value) return;

  terminal = new Terminal({
    disableStdin: true,
    scrollback: settingsStore.maxLogLines,
    fontSize: 13,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
    theme: {
      background: "#111827",
      foreground: "#d1d5db",
      cursor: "#d1d5db",
      selectionBackground: "#374151",
    },
    convertEol: true,
  });

  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);

  const webLinksAddon = new WebLinksAddon((_event, uri) => {
    open(uri);
  });
  terminal.loadAddon(webLinksAddon);

  searchAddon = new SearchAddon();
  terminal.loadAddon(searchAddon);

  terminal.open(terminalRef.value);

  // Prevent terminal from capturing Ctrl+F
  terminal.attachCustomKeyEventHandler((e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "f") {
      return false; // Let the window handler take it
    }
    return true;
  });

  // Register custom file path link provider
  terminal.registerLinkProvider({
    provideLinks(bufferLineNumber: number, callback: (links: Array<{ range: { start: { x: number; y: number }; end: { x: number; y: number } }; text: string; activate: () => void }> | undefined) => void) {
      if (!terminal) { callback(undefined); return; }
      const line = terminal.buffer.active.getLine(bufferLineNumber - 1);
      if (!line) { callback(undefined); return; }
      const lineText = line.translateToString();

      // Match file paths with optional line:col - e.g. src/main.ts:42:10 or /home/user/file.rs:5
      const FILE_EXTS = "(?:ts|tsx|js|jsx|vue|rs|py|go|java|kt|rb|c|cpp|h|hpp|cs|swift|json|yaml|yml|toml|css|scss|html|xml|sql|sh|bash|zsh|md|txt|log|cfg|conf|ini|env)";
      const filePattern = new RegExp(
        `(?:^|[\\s('"\\[])(((?:\\./|\\.\\./)(?:[\\w.-]+/)*[\\w.-]+\\.${FILE_EXTS}|/(?:[\\w.-]+/)+[\\w.-]+\\.${FILE_EXTS}|(?:[a-zA-Z][\\w.-]*/)(?:[\\w.-]+/)*[\\w.-]+\\.${FILE_EXTS}))(?::(\\d+))?(?::(\\d+))?`,
        "g",
      );
      // Match absolute directory paths (e.g., /home/user/project)
      const dirPattern = /(?:^|[\s('"[])((\/(?:[\w.-]+\/)+[\w.-]+))(?=[\s)'":\],$]|$)/g;

      const links: Array<{ range: { start: { x: number; y: number }; end: { x: number; y: number } }; text: string; activate: () => void }> = [];
      const usedRanges: Array<{ start: number; end: number }> = [];

      // Match file paths with extensions
      let match;
      while ((match = filePattern.exec(lineText)) !== null) {
        const fullMatch = match[0];
        const filePath = match[1];
        const lineNum = match[3] ? parseInt(match[3]) : undefined;
        const colNum = match[4] ? parseInt(match[4]) : undefined;

        const matchStart = match.index + fullMatch.indexOf(filePath);
        const suffixLength = fullMatch.length - fullMatch.indexOf(filePath);
        const startX = matchStart + 1;
        const endX = matchStart + suffixLength;

        usedRanges.push({ start: matchStart, end: matchStart + suffixLength });
        links.push({
          range: {
            start: { x: startX, y: bufferLineNumber },
            end: { x: endX, y: bufferLineNumber },
          },
          text: filePath,
          activate: () => {
            openFilePath(filePath, lineNum, colNum);
          },
        });
      }

      // Match directory paths (avoid overlapping with file paths)
      while ((match = dirPattern.exec(lineText)) !== null) {
        const fullMatch = match[0];
        const dirPath = match[1];
        const matchStart = match.index + fullMatch.indexOf(dirPath);
        const matchEnd = matchStart + dirPath.length;

        // Skip if overlaps with a file path
        const overlaps = usedRanges.some(
          (r) => (matchStart >= r.start && matchStart < r.end) || (matchEnd > r.start && matchEnd <= r.end),
        );
        if (overlaps) continue;

        const startX = matchStart + 1;
        const endX = matchEnd;

        links.push({
          range: {
            start: { x: startX, y: bufferLineNumber },
            end: { x: endX, y: bufferLineNumber },
          },
          text: dirPath,
          activate: () => {
            openFilePath(dirPath);
          },
        });
      }

      callback(links.length > 0 ? links : undefined);
    },
  });

  try {
    fitAddon.fit();
  } catch {
    // Terminal not visible yet
  }

  await writeExistingLogs();

  unlisten = await listen<LogMessage>("process-log", (event) => {
    if (event.payload.projectId === props.projectId) {
      scheduleWrite(event.payload.data);
    }
  });
}

function handleResize() {
  try {
    fitAddon?.fit();
  } catch {
    // Ignore
  }
}

async function clearTerminal() {
  terminal?.clear();
  logs.clearProjectLogs(props.projectId);
  try {
    await invoke("clear_project_logs", { projectId: props.projectId });
  } catch {
    // Ignore
  }
  emit("clear");
}

onMounted(() => {
  setupTerminal();
  window.addEventListener("resize", handleResize);
  window.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  window.removeEventListener("keydown", handleKeydown);
  if (rafId !== null) cancelAnimationFrame(rafId);
  unlisten?.();
  terminal?.dispose();
  terminal = null;
  fitAddon = null;
  searchAddon = null;
});

watch(
  () => props.projectId,
  async () => {
    if (terminal) {
      terminal.clear();
      terminal.reset();
      await writeExistingLogs();
    }
    showSearch.value = false;
    searchQuery.value = "";
  },
);

defineExpose({ clearTerminal });
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0">
    <div class="flex items-center justify-between px-3 py-1.5 border-b border-gray-700">
      <span class="text-xs text-gray-400 uppercase tracking-wide">Output</span>
      <div class="flex items-center gap-2">
        <button
          class="text-xs text-gray-500 hover:text-gray-300 transition-colors"
          title="Search (Ctrl+F)"
          @click="toggleSearch"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        </button>
        <button
          class="text-xs text-gray-500 hover:text-gray-300 transition-colors"
          @click="clearTerminal"
        >
          Clear
        </button>
      </div>
    </div>

    <!-- Search bar -->
    <div v-if="showSearch" class="flex items-center gap-2 px-3 py-1.5 border-b border-gray-700 bg-gray-800">
      <input
        ref="searchInputRef"
        v-model="searchQuery"
        placeholder="Search..."
        class="flex-1 px-2 py-1 bg-gray-900 border border-gray-600 rounded text-gray-100 text-xs focus:outline-none focus:border-blue-500"
        @input="onSearchInput"
        @keydown="onSearchKeydown"
      />
      <button
        class="p-1 text-gray-400 hover:text-gray-200 transition-colors"
        title="Previous (Shift+Enter)"
        @click="findPrevious"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
        </svg>
      </button>
      <button
        class="p-1 text-gray-400 hover:text-gray-200 transition-colors"
        title="Next (Enter)"
        @click="findNext"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>
      <button
        class="p-1 text-gray-400 hover:text-gray-200 transition-colors"
        title="Close"
        @click="toggleSearch"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <div ref="terminalRef" class="flex-1 min-h-0 p-1"></div>
  </div>
</template>
