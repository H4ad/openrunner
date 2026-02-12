<script setup lang="ts">
import { ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
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
import { FileTextIcon } from "@radix-icons/vue";

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

function handleOpenChange(open: boolean) {
  if (!open) {
    emit("cancel");
  }
}
</script>

<template>
  <Dialog :open="props.open" @update:open="handleOpenChange">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>{{ props.title }}</DialogTitle>
      </DialogHeader>
      <form @submit.prevent="submit" class="space-y-4">
        <div class="space-y-2">
          <Label for="primary-input">{{ props.label }}</Label>
          <div class="flex gap-2">
            <Input
              id="primary-input"
              v-model="inputValue"
              :placeholder="props.placeholder"
              class="flex-1"
              autofocus
            />
            <Button
              v-if="props.browseDirectory"
              type="button"
              variant="outline"
              size="icon"
              title="Browse folder"
              @click="browseFolder('primary')"
            >
              <FileTextIcon class="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div v-if="props.secondaryLabel" class="space-y-2">
          <Label for="secondary-input">{{ props.secondaryLabel }}</Label>
          <div class="flex gap-2">
            <Input
              id="secondary-input"
              v-model="secondaryInputValue"
              :placeholder="props.secondaryPlaceholder"
              class="flex-1"
            />
            <Button
              v-if="props.secondaryBrowseDirectory"
              type="button"
              variant="outline"
              size="icon"
              title="Browse folder"
              @click="browseFolder('secondary')"
            >
              <FileTextIcon class="h-4 w-4" />
            </Button>
          </div>
        </div>

        <DialogFooter class="flex flex-row justify-end gap-2">
          <Button type="button" variant="secondary" @click="emit('cancel')">
            Cancel
          </Button>
          <Button type="submit" :disabled="!inputValue.trim()"> Save </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
