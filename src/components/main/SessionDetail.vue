<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { open } from "@tauri-apps/plugin-shell";
import { invoke } from "@tauri-apps/api/core";
import "@xterm/xterm/css/xterm.css";
import { Line } from "vue-chartjs";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Filler,
} from "chart.js";
import { useSessionsStore } from "../../stores/sessions";
import { useUiStore } from "../../stores/ui";
import type { MetricPoint } from "../../types";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Filler,
);

const props = defineProps<{
  sessionId: string;
}>();

const sessions = useSessionsStore();
const ui = useUiStore();
const terminalRef = ref<HTMLDivElement>();
const loading = ref(true);
const showCharts = ref(true);
const metrics = ref<MetricPoint[]>([]);
const projectId = ref<string | null>(null);

// Search state
const showSearch = ref(false);
const searchQuery = ref("");
const searchInputRef = ref<HTMLInputElement>();

let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let searchAddon: SearchAddon | null = null;

// Chart data
const cpuChartData = shallowRef({
  labels: [] as string[],
  datasets: [
    {
      label: "CPU %",
      data: [] as number[],
      borderColor: "#3b82f6",
      backgroundColor: "rgba(59,130,246,0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 0,
      borderWidth: 1.5,
    },
  ],
});

const memChartData = shallowRef({
  labels: [] as string[],
  datasets: [
    {
      label: "Memory (MB)",
      data: [] as number[],
      borderColor: "#10b981",
      backgroundColor: "rgba(16,185,129,0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 0,
      borderWidth: 1.5,
    },
  ],
});

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  animation: { duration: 0 },
  scales: {
    x: {
      display: false,
    },
    y: {
      beginAtZero: true,
      grid: { color: "rgba(75,85,99,0.3)" },
      ticks: { color: "#9ca3af", font: { size: 10 } },
    },
  },
  plugins: {
    tooltip: {
      enabled: true,
      mode: "index" as const,
      intersect: false,
    },
  },
};

async function loadMetrics() {
  try {
    metrics.value = await sessions.getSessionMetrics(props.sessionId);
    if (metrics.value.length > 0) {
      cpuChartData.value = {
        labels: metrics.value.map((p) =>
          new Date(p.timestamp).toLocaleTimeString(),
        ),
        datasets: [
          {
            ...cpuChartData.value.datasets[0],
            data: metrics.value.map((p) =>
              parseFloat(p.cpuUsage.toFixed(1)),
            ),
          },
        ],
      };
      memChartData.value = {
        labels: metrics.value.map((p) =>
          new Date(p.timestamp).toLocaleTimeString(),
        ),
        datasets: [
          {
            ...memChartData.value.datasets[0],
            data: metrics.value.map((p) =>
              parseFloat((p.memoryUsage / (1024 * 1024)).toFixed(1)),
            ),
          },
        ],
      };
    }
  } catch {
    // No metrics for this session
  }
}

// Search functions
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

// File path detection
async function openFilePath(filePath: string, line?: number, column?: number) {
  try {
    let workingDir = "";
    if (projectId.value) {
      workingDir = await invoke<string>("resolve_working_dir_by_project", {
        projectId: projectId.value,
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

async function loadSessionInfo() {
  try {
    const session = await invoke<{ projectId: string } | null>("get_session", {
      sessionId: props.sessionId,
    });
    if (session) {
      projectId.value = session.projectId;
    }
  } catch {
    // Ignore
  }
}

async function setupTerminal() {
  if (!terminalRef.value) return;

  terminal = new Terminal({
    disableStdin: true,
    scrollback: 50000,
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

      const FILE_EXTS = "(?:ts|tsx|js|jsx|vue|rs|py|go|java|kt|rb|c|cpp|h|hpp|cs|swift|json|yaml|yml|toml|css|scss|html|xml|sql|sh|bash|zsh|md|txt|log|cfg|conf|ini|env)";
      // Match files with extensions (with optional line:col) OR absolute directory paths
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

  loading.value = true;
  try {
    const logs = await sessions.getSessionLogs(props.sessionId);
    if (logs) {
      terminal.write(logs);
    }
  } finally {
    loading.value = false;
  }
}

function handleResize() {
  try {
    fitAddon?.fit();
  } catch {
    // Ignore
  }
}

onMounted(() => {
  loadSessionInfo();
  setupTerminal();
  loadMetrics();
  window.addEventListener("resize", handleResize);
  window.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  window.removeEventListener("keydown", handleKeydown);
  terminal?.dispose();
  terminal = null;
  fitAddon = null;
  searchAddon = null;
});
</script>

<template>
  <div class="flex-1 flex flex-col h-full min-h-0">
    <div class="p-4 border-b border-gray-700 flex items-center gap-3">
      <button
        class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
        @click="ui.backToProject()"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </button>
      <h2 class="text-lg font-semibold text-gray-100">Session Logs</h2>
      <span v-if="loading" class="text-xs text-gray-500">Loading...</span>
      <div class="ml-auto flex items-center gap-2">
        <button
          v-if="metrics.length > 0"
          class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
          :class="showCharts ? 'bg-gray-700 text-gray-200' : ''"
          title="Toggle Charts"
          @click="showCharts = !showCharts"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
        </button>
        <button
          class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
          title="Search (Ctrl+F)"
          @click="toggleSearch"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Charts -->
    <div v-if="showCharts && metrics.length > 0" class="border-b border-gray-700 px-4 py-3">
      <div class="grid grid-cols-2 gap-4">
        <div>
          <h4 class="text-xs text-gray-400 mb-1">CPU Usage</h4>
          <div class="h-24">
            <Line :data="cpuChartData" :options="chartOptions" />
          </div>
        </div>
        <div>
          <h4 class="text-xs text-gray-400 mb-1">Memory Usage (MB)</h4>
          <div class="h-24">
            <Line :data="memChartData" :options="chartOptions" />
          </div>
        </div>
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
