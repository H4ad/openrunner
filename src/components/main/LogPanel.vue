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
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  MagnifyingGlassIcon,
  ChevronUpIcon,
  ChevronDownIcon,
  Cross1Icon,
} from "@radix-icons/vue";

const props = defineProps<{
  projectId: string;
  groupId?: string;
  interactive: boolean;
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
  if (props.groupId) {
    try {
      const data = await invoke<string>("read_project_logs", {
        groupId: props.groupId,
        projectId: props.projectId,
      });
      if (data) {
        terminal.write(data);
      }
    } catch {
      // No saved logs
    }
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
    disableStdin: !props.interactive,
    scrollback: settingsStore.maxLogLines,
    fontSize: 13,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
    theme: {
      background: "hsl(var(--background))",
      foreground: "hsl(var(--foreground))",
      cursor: "hsl(var(--foreground))",
      selectionBackground: "hsl(var(--accent))",
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

  // Enable PTY interaction if interactive mode
  if (props.interactive) {
    terminal.onData(async (data) => {
      try {
        await invoke("write_to_process_stdin", {
          projectId: props.projectId,
          data,
        });
      } catch {
        // Process might not be running
      }
    });

    terminal.onResize(async ({ cols, rows }) => {
      try {
        await invoke("resize_pty", {
          projectId: props.projectId,
          cols,
          rows,
        });
      } catch {
        // Process might not be running
      }
    });
  }

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

      // Match file paths with optional line:col
      const FILE_EXTS = "(?:ts|tsx|js|jsx|vue|rs|py|go|java|kt|rb|c|cpp|h|hpp|cs|swift|json|yaml|yml|toml|css|scss|html|xml|sql|sh|bash|zsh|md|txt|log|cfg|conf|ini|env)";
      const filePattern = new RegExp(
        `(?:^|[\s('"\\[\[])(((?:\./|\.\./)(?:[\w.-]+/)*[\w.-]+\.${FILE_EXTS}|/(?:[\w.-]+/)+[\w.-]+\.${FILE_EXTS}|(?:[a-zA-Z][\w.-]*/)(?:[\w.-]+/)*[\w.-]+\.${FILE_EXTS}))(?::(\d+))?(?::(\d+))?`,
        "g",
      );
      const dirPattern = /(?:^|[\s('"[\[])((\/(?:[\w.-]+\/)+[\w.-]+))(?=[\s)'":\],$]|$)/g;

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
  if (props.groupId) {
    try {
      await invoke("clear_project_logs", { 
        groupId: props.groupId,
        projectId: props.projectId 
      });
    } catch {
      // Ignore
    }
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
    <div class="flex items-center justify-between px-3 py-1.5 border-b border-border">
      <div class="flex items-center gap-2">
        <span class="text-xs text-muted-foreground uppercase tracking-wide">Output</span>
        <span v-if="props.interactive" class="text-xs px-1.5 py-0.5 rounded bg-green-500/20 text-green-500 font-medium">PTY</span>
      </div>
      <div class="flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon"
          class="h-6 w-6"
          title="Search (Ctrl+F)"
          @click="toggleSearch"
        >
          <MagnifyingGlassIcon class="h-3.5 w-3.5" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="text-xs h-6"
          @click="clearTerminal"
        >
          Clear
        </Button>
      </div>
    </div>

    <!-- Search bar -->
    <div v-if="showSearch" class="flex items-center gap-2 px-3 py-1.5 border-b border-border bg-muted">
      <Input
        ref="searchInputRef"
        v-model="searchQuery"
        placeholder="Search..."
        class="flex-1 h-7 text-xs"
        @input="onSearchInput"
        @keydown="onSearchKeydown"
      />
      <Button variant="ghost" size="icon" class="h-7 w-7" title="Previous (Shift+Enter)" @click="findPrevious">
        <ChevronUpIcon class="h-3.5 w-3.5" />
      </Button>
      <Button variant="ghost" size="icon" class="h-7 w-7" title="Next (Enter)" @click="findNext">
        <ChevronDownIcon class="h-3.5 w-3.5" />
      </Button>
      <Button variant="ghost" size="icon" class="h-7 w-7" title="Close" @click="toggleSearch">
        <Cross1Icon class="h-3.5 w-3.5" />
      </Button>
    </div>

    <div ref="terminalRef" class="flex-1 min-h-0 p-1"></div>
  </div>
</template>
