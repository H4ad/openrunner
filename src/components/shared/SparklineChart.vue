<script setup lang="ts">
import { computed } from "vue";
import { Line } from "vue-chartjs";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Tooltip,
  Filler,
} from "chart.js";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Tooltip,
  Filler,
);

const props = defineProps<{
  data: number[];
  color?: string;
  fillColor?: string;
  height?: number;
}>();

const chartData = computed(() => ({
  labels: props.data.map((_, i) => i.toString()),
  datasets: [
    {
      data: props.data,
      borderColor: props.color || "#3b82f6",
      backgroundColor: props.fillColor || "rgba(59,130,246,0.1)",
      fill: true,
      tension: 0.4,
      pointRadius: 0,
      borderWidth: 1.5,
    },
  ],
}));

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  animation: { duration: 0 },
  scales: {
    x: { display: false },
    y: { display: false, min: 0 },
  },
  plugins: {
    tooltip: { enabled: false },
    legend: { display: false },
  },
  interaction: {
    intersect: false,
    mode: "index" as const,
  },
};
</script>

<template>
  <div :style="{ height: height ? `${height}px` : '24px' }">
    <Line :data="chartData" :options="chartOptions" />
  </div>
</template>
