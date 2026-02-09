<script setup lang="ts">
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

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
        <DialogDescription>{{ props.message }}</DialogDescription>
      </DialogHeader>
      <DialogFooter class="flex flex-row justify-end gap-2">
        <Button variant="secondary" @click="emit('cancel')"> Cancel </Button>
        <Button variant="destructive" @click="emit('confirm')">
          {{ props.confirmLabel ?? "Delete" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
