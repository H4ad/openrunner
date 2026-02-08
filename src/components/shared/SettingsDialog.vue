<script setup lang="ts">
import { ref, watch } from "vue";
import { useSettingsStore } from "../../stores/settings";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const settings = useSettingsStore();
const maxLogLines = ref(settings.maxLogLines);

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      maxLogLines.value = settings.maxLogLines;
    }
  },
);

async function save() {
  await settings.updateMaxLogLines(maxLogLines.value);
  emit("close");
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="emit('close')"
    >
      <div class="bg-gray-800 rounded-lg shadow-xl p-6 w-96 border border-gray-700">
        <h3 class="text-lg font-semibold text-gray-100 mb-4">Settings</h3>
        <form @submit.prevent="save">
          <label class="block text-sm text-gray-400 mb-1">Max Log Lines</label>
          <input
            v-model.number="maxLogLines"
            type="number"
            min="1000"
            max="100000"
            step="1000"
            class="w-full px-3 py-2 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500 mb-1"
          />
          <p class="text-xs text-gray-500 mb-4">
            Number of log lines to keep in memory per project (1,000 - 100,000).
          </p>
          <div class="flex justify-end gap-3">
            <button
              type="button"
              class="px-4 py-2 text-sm rounded bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors"
              @click="emit('close')"
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
