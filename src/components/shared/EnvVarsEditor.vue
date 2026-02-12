<script setup lang="ts">
import { ref, watch } from "vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Cross1Icon, PlusIcon } from "@radix-icons/vue";

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

function handleOpenChange(open: boolean) {
  if (!open) {
    emit("cancel");
  }
}
</script>

<template>
  <Dialog :open="props.open" @update:open="handleOpenChange">
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>{{ props.title }}</DialogTitle>
      </DialogHeader>
      <form @submit.prevent="submit" class="space-y-4">
        <div class="grid grid-cols-[1fr_1fr_auto] gap-2 text-xs text-muted-foreground px-1">
          <Label>Key</Label>
          <Label>Value</Label>
          <span></span>
        </div>

        <ScrollArea class="h-64">
          <div class="space-y-2 pr-4">
            <div v-for="(row, index) in rows" :key="index" class="grid grid-cols-[1fr_1fr_auto] gap-2">
              <Input
                v-model="row.key"
                placeholder="ENV_NAME"
                class="font-mono text-sm"
              />
              <Input
                v-model="row.value"
                placeholder="value"
                class="font-mono text-sm"
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                class="shrink-0 text-muted-foreground hover:text-destructive"
                title="Remove"
                @click="removeRow(index)"
              >
                <Cross1Icon class="h-4 w-4" />
              </Button>
            </div>
          </div>
        </ScrollArea>

        <Button type="button" variant="ghost" size="sm" class="text-primary" @click="addRow">
          <PlusIcon class="h-4 w-4 mr-1" />
          Add Variable
        </Button>

        <DialogFooter class="flex flex-row justify-end gap-2">
          <Button type="button" variant="secondary" @click="emit('cancel')">
            Cancel
          </Button>
          <Button type="submit">Save</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
