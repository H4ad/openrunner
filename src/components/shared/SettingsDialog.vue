<script setup lang="ts">
import { ref, watch } from "vue";
import { useSettingsStore } from "../../stores/settings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

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

function handleOpenChange(open: boolean) {
  if (!open) {
    emit("close");
  }
}
</script>

<template>
  <Dialog :open="props.open" @update:open="handleOpenChange">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>Settings</DialogTitle>
      </DialogHeader>
      <form @submit.prevent="save" class="space-y-4">
        <div class="space-y-2">
          <Label for="max-log-lines">Max Log Lines</Label>
          <Input
            id="max-log-lines"
            v-model.number="maxLogLines"
            type="number"
            min="1000"
            max="100000"
            step="1000"
          />
          <p class="text-xs text-muted-foreground">
            Number of log lines to keep in memory per project (1,000 - 100,000).
          </p>
        </div>

        <DialogFooter class="flex flex-row justify-end gap-2">
          <Button type="button" variant="secondary" @click="emit('close')">
            Cancel
          </Button>
          <Button type="submit">Save</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
