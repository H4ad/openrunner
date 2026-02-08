<script setup lang="ts">
const props = defineProps<{
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
}>();

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="emit('cancel')"
    >
      <div class="bg-gray-800 rounded-lg shadow-xl p-6 w-96 border border-gray-700">
        <h3 class="text-lg font-semibold text-gray-100 mb-2">
          {{ props.title }}
        </h3>
        <p class="text-gray-400 text-sm mb-6">{{ props.message }}</p>
        <div class="flex justify-end gap-3">
          <button
            class="px-4 py-2 text-sm rounded bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors"
            @click="emit('cancel')"
          >
            Cancel
          </button>
          <button
            class="px-4 py-2 text-sm rounded bg-red-600 text-white hover:bg-red-500 transition-colors"
            @click="emit('confirm')"
          >
            {{ props.confirmLabel ?? "Delete" }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
