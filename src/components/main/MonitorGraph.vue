<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
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
import { listen, type UnlistenFn } from "@/lib/api";
import type { ProcessInfo, MetricPoint } from "../../types";

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
  projectId: string;
  initialData?: MetricPoint[];
}>();

const MAX_POINTS = 60;

const cpuData = ref<number[]>([]);
const memData = ref<number[]>([]);
const labels = ref<string[]>([]);

let unlisten: UnlistenFn | null = null;

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

function updateCharts() {
  cpuChartData.value = {
    labels: [...labels.value],
    datasets: [
      {
        ...cpuChartData.value.datasets[0],
        data: [...cpuData.value],
      },
    ],
  };

  memChartData.value = {
    labels: [...labels.value],
    datasets: [
      {
        ...memChartData.value.datasets[0],
        data: [...memData.value],
      },
    ],
  };
}

function loadInitialData() {
  if (!props.initialData || props.initialData.length === 0) return;

  labels.value = props.initialData.map((p) =>
    new Date(p.timestamp).toLocaleTimeString(),
  );
  cpuData.value = props.initialData.map((p) =>
    parseFloat(p.cpuUsage.toFixed(1)),
  );
  memData.value = props.initialData.map((p) =>
    parseFloat((p.memoryUsage / (1024 * 1024)).toFixed(1)),
  );

  // Trim to MAX_POINTS
  if (labels.value.length > MAX_POINTS) {
    labels.value = labels.value.slice(-MAX_POINTS);
    cpuData.value = cpuData.value.slice(-MAX_POINTS);
    memData.value = memData.value.slice(-MAX_POINTS);
  }

  updateCharts();
}

onMounted(async () => {
  loadInitialData();

  unlisten = await listen<ProcessInfo[]>("process-stats-updated", (payload) => {
    const info = payload.find((i) => i.projectId === props.projectId);
    if (!info) return;

    const now = new Date().toLocaleTimeString();
    labels.value.push(now);
    cpuData.value.push(parseFloat(info.cpuUsage.toFixed(1)));
    memData.value.push(
      parseFloat((info.memoryUsage / (1024 * 1024)).toFixed(1)),
    );

    if (labels.value.length > MAX_POINTS) {
      labels.value.shift();
      cpuData.value.shift();
      memData.value.shift();
    }

    updateCharts();
  });
});

onUnmounted(() => {
  unlisten?.();
});

watch(
  () => props.initialData,
  () => {
    if (props.initialData) {
      cpuData.value = [];
      memData.value = [];
      labels.value = [];
      loadInitialData();
    }
  },
);
</script>

<template>
  <div class="border-b border-gray-700 px-4 py-3">
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
</template>
