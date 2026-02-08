<script setup lang="ts">
import { ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";

const props = defineProps<{
  open: boolean;
  title: string;
  label: string;
  value?: string;
  placeholder?: string;
  secondaryLabel?: string;
  secondaryValue?: string;
  secondaryPlaceholder?: string;
  browseDirectory?: boolean;
  secondaryBrowseDirectory?: boolean;
}>();

const emit = defineEmits<{
  confirm: [value: string, secondaryValue?: string];
  cancel: [];
}>();

const inputValue = ref("");
const secondaryInputValue = ref("");

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      inputValue.value = props.value ?? "";
      secondaryInputValue.value = props.secondaryValue ?? "";
    }
  },
);

async function browseFolder(target: "primary" | "secondary") {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select Folder",
  });
  if (selected) {
    if (target === "primary") {
      inputValue.value = selected as string;
    } else {
      secondaryInputValue.value = selected as string;
    }
  }
}

function submit() {
  if (!inputValue.value.trim()) return;
  emit("confirm", inputValue.value.trim(), secondaryInputValue.value.trim() || undefined);
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="emit('cancel')"
    >
      <div class="bg-gray-800 rounded-lg shadow-xl p-6 w-96 border border-gray-700">
        <h3 class="text-lg font-semibold text-gray-100 mb-4">
          {{ props.title }}
        </h3>
        <form @submit.prevent="submit">
          <label class="block text-sm text-gray-400 mb-1">{{ props.label }}</label>
          <div class="flex gap-2 mb-4">
            <input
              v-model="inputValue"
              :placeholder="props.placeholder"
              class="flex-1 px-3 py-2 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500"
              autofocus
            />
            <button
              v-if="props.browseDirectory"
              type="button"
              class="px-2 py-2 text-xs rounded bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors flex-shrink-0"
              title="Browse folder"
              @click="browseFolder('primary')"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
              </svg>
            </button>
          </div>
          <template v-if="props.secondaryLabel">
            <label class="block text-sm text-gray-400 mb-1">{{ props.secondaryLabel }}</label>
            <div class="flex gap-2 mb-4">
              <input
                v-model="secondaryInputValue"
                :placeholder="props.secondaryPlaceholder"
                class="flex-1 px-3 py-2 bg-gray-900 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:border-blue-500"
              />
              <button
                v-if="props.secondaryBrowseDirectory"
                type="button"
                class="px-2 py-2 text-xs rounded bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors flex-shrink-0"
                title="Browse folder"
                @click="browseFolder('secondary')"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                </svg>
              </button>
            </div>
          </template>
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
              :disabled="!inputValue.trim()"
            >
              Save
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
