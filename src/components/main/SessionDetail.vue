<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { open } from "@/lib/shell";
import { invoke } from "@/lib/api";
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
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  ArrowLeftIcon,
  ActivityLogIcon,
  MagnifyingGlassIcon,
  ChevronUpIcon,
  ChevronDownIcon,
  Cross1Icon,
} from "@radix-icons/vue";

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
  groupId: string;
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
    metrics.value = await sessions.getSessionMetrics(props.groupId, props.sessionId);
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
      groupId: props.groupId,
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

  // Custom key handler for copy and search
  terminal.attachCustomKeyEventHandler((e) => {
    // Ctrl+F / Cmd+F for search - let the window handler take it
    if ((e.ctrlKey || e.metaKey) && e.key === "f") {
      return false;
    }
    
    // Ctrl+C / Cmd+C - copy selection to clipboard if text is selected
    if ((e.ctrlKey || e.metaKey) && e.key === "c" && e.type === "keydown") {
      if (terminal?.hasSelection()) {
        const selection = terminal.getSelection();
        if (selection) {
          navigator.clipboard.writeText(selection);
        }
      }
      return false; // Session detail is read-only, always prevent default
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
      const filePattern = new RegExp(
        `(?:^[\s('"\\[\[])(((?:\./|\.\./)(?:[\w.-]+/)*[\w.-]+\.${FILE_EXTS}|/(?:[\w.-]+/)+[\w.-]+\.${FILE_EXTS}|(?:[a-zA-Z][\w.-]*/)(?:[\w.-]+/)*[\w.-]+\.${FILE_EXTS}))(?::(\d+))?(?::(\d+))?`,
        "g",
      );
      const dirPattern = /(?:^|[\s('"[\[])((\/(?:[\w.-]+\/)+[\w.-]+))(?=[\s)'":\],$]|$)/g;

      const links: Array<{ range: { start: { x: number; y: number }; end: { x: number; y: number } }; text: string; activate: () => void }> = [];
      const usedRanges: Array<{ start: number; end: number }> = [];

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

      while ((match = dirPattern.exec(lineText)) !== null) {
        const fullMatch = match[0];
        const dirPath = match[1];
        const matchStart = match.index + fullMatch.indexOf(dirPath);
        const matchEnd = matchStart + dirPath.length;

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
    const logs = await sessions.getSessionLogs(props.groupId, props.sessionId);
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
    <div class="p-4 border-b border-border flex items-center gap-3">
      <Button variant="ghost" size="icon" class="h-8 w-8" @click="ui.backToProject()">
        <ArrowLeftIcon class="h-4 w-4" />
      </Button>
      <h2 class="text-lg font-semibold text-foreground">Session Logs</h2>
      <span v-if="loading" class="text-xs text-muted-foreground">Loading...</span>
      <div class="ml-auto flex items-center gap-2">
        <Button
          v-if="metrics.length > 0"
          variant="ghost"
          size="icon"
          class="h-8 w-8"
          :class="showCharts ? 'bg-accent text-accent-foreground' : ''"
          title="Toggle Charts"
          @click="showCharts = !showCharts"
        >
          <ActivityLogIcon class="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          class="h-8 w-8"
          title="Search (Ctrl+F)"
          @click="toggleSearch"
        >
          <MagnifyingGlassIcon class="h-4 w-4" />
        </Button>
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
