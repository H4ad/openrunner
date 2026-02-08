<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{
  open: boolean;
  title: string;
  envVars: Record<string, string>;
}>();

const emit = defineEmits<{
  confirm: [envVars: Record<string, string>];
  cancel: [];
}>();

const rows = ref<{ key: string; value: string }[]>([]);

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      const entries = Object.entries(props.envVars);
      rows.value =
        entries.length > 0
          ? entries.map(([key, value]) => ({ key, value }))
          : [{ key: "", value: "" }];
    }
  },
);

function addRow() {
  rows.value.push({ key: "", value: "" });
}

function removeRow(index: number) {
  rows.value.splice(index, 1);
  if (rows.value.length === 0) {
    rows.value.push({ key: "", value: "" });
  }
}

function submit() {
  const result: Record<string, string> = {};
  for (const row of rows.value) {
    const key = row.key.trim();
    if (key) {
      result[key] = row.value;
    }
  }
  emit("confirm", result);
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="emit('cancel')"
    >
      <div class="bg-gray-800 rounded-lg shadow-xl p-6 w-[32rem] border border-gray-700">
        <h3 class="text-lg font-semibold text-gray-100 mb-4">
          {{ props.title }}
        </h3>
        <form @submit.prevent="submit">
          <div class="space-y-2 mb-4 max-h-64 overflow-y-auto">
            <div class="flex items-center gap-2 text-xs text-gray-500 px-1">
              <span class="flex-1">Key</span>
              <span class="flex-1">Value</span>
              <span class="w-7"></span>
            </div>
            <div
              v-for="(row, index) in rows"
              :key="index"
              class="flex items-center gap-2"
            >
              <input
                v-model="row.key"
                placeholder="ENV_NAME"
                class="flex-1 px-2 py-1.5 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500 font-mono"
              />
              <input
                v-model="row.value"
                placeholder="value"
                class="flex-1 px-2 py-1.5 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500 font-mono"
              />
              <button
                type="button"
                class="p-1 text-gray-500 hover:text-red-400 transition-colors flex-shrink-0"
                title="Remove"
                @click="removeRow(index)"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>
          <button
            type="button"
            class="text-xs text-blue-400 hover:text-blue-300 transition-colors mb-4"
            @click="addRow"
          >
            + Add Variable
          </button>
          <div class="flex justify-end gap-3">
            <button
              type="button"
              class="px-4 py-2 text-sm rounded bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors"
              @click="emit('cancel')"
            >
              Cancel
            </button>
            <button
              type="submit"
              class="px-4 py-2 text-sm rounded bg-blue-600 text-white hover:bg-blue-500 transition-colors"
            >
              Save
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
