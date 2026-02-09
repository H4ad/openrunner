<script setup lang="ts">
import { ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectType } from "../../types";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { FileTextIcon, PlusIcon, Cross1Icon } from "@radix-icons/vue";

const props = defineProps<{
  open: boolean;
  title: string;
  name?: string;
  command?: string;
  cwd?: string;
  envVars?: Record<string, string>;
  projectType?: ProjectType;
}>();

const emit = defineEmits<{
  confirm: [
    name: string,
    command: string,
    cwd: string | undefined,
    envVars: Record<string, string>,
    projectType: ProjectType,
  ];
  cancel: [];
}>();

const nameValue = ref("");
const commandValue = ref("");
const cwdValue = ref("");
const projectTypeValue = ref<ProjectType>("service");
const envRows = ref<{ key: string; value: string }[]>([]);

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      nameValue.value = props.name ?? "";
      commandValue.value = props.command ?? "";
      cwdValue.value = props.cwd ?? "";
      projectTypeValue.value = props.projectType ?? "service";
      const entries = Object.entries(props.envVars ?? {});
      envRows.value =
        entries.length > 0
          ? entries.map(([key, value]) => ({ key, value }))
          : [];
    }
  },
);

async function browseFolder() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select Working Directory",
  });
  if (selected) {
    cwdValue.value = selected as string;
  }
}

function addEnvRow() {
  envRows.value.push({ key: "", value: "" });
}

function removeEnvRow(index: number) {
  envRows.value.splice(index, 1);
}

function submit() {
  if (!nameValue.value.trim() || !commandValue.value.trim()) return;
  const envVarsResult: Record<string, string> = {};
  for (const row of envRows.value) {
    const key = row.key.trim();
    if (key) {
      envVarsResult[key] = row.value;
    }
  }
  emit(
    "confirm",
    nameValue.value.trim(),
    commandValue.value.trim(),
    cwdValue.value.trim() || undefined,
    envVarsResult,
    projectTypeValue.value,
  );
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
        <div class="space-y-2">
          <Label for="project-name">Project Name</Label>
          <Input
            id="project-name"
            v-model="nameValue"
            placeholder="My Project"
            autofocus
          />
        </div>

        <div class="space-y-2">
          <Label for="project-command">Command</Label>
          <Input
            id="project-command"
            v-model="commandValue"
            placeholder="npm run dev"
          />
        </div>

        <div class="space-y-2">
          <Label for="project-type">Type</Label>
          <Select v-model="projectTypeValue">
            <SelectTrigger id="project-type">
              <SelectValue placeholder="Select type" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="service">Service</SelectItem>
              <SelectItem value="task">Task</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div class="space-y-2">
          <Label for="project-cwd">
            Working Directory
            <span class="text-muted-foreground text-xs">(optional, relative to group)</span>
          </Label>
          <div class="flex gap-2">
            <Input
              id="project-cwd"
              v-model="cwdValue"
              placeholder="packages/my-app"
              class="flex-1"
            />
            <Button
              type="button"
              variant="outline"
              size="icon"
              title="Browse folder"
              @click="browseFolder"
            >
              <FileTextIcon class="h-4 w-4" />
            </Button>
          </div>
        </div>

        <!-- Environment Variables -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <Label>Environment Variables</Label>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              class="text-primary"
              @click="addEnvRow"
            >
              <PlusIcon class="h-4 w-4 mr-1" />
              Add
            </Button>
          </div>

          <ScrollArea v-if="envRows.length > 0" class="h-32">
            <div class="space-y-2 pr-4">
              <div
                v-for="(row, index) in envRows"
                :key="index"
                class="grid grid-cols-[1fr_1fr_auto] gap-2"
              >
                <Input
                  v-model="row.key"
                  placeholder="KEY"
                  class="font-mono text-xs"
                />
                <Input
                  v-model="row.value"
                  placeholder="value"
                  class="font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  class="shrink-0 text-muted-foreground hover:text-destructive h-8 w-8"
                  @click="removeEnvRow(index)"
                >
                  <Cross1Icon class="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
          </ScrollArea>
          <p v-else class="text-xs text-muted-foreground">No environment variables set.</p>
        </div>

        <DialogFooter class="flex flex-row justify-end gap-2">
          <Button type="button" variant="secondary" @click="emit('cancel')">
            Cancel
          </Button>
          <Button
            type="submit"
            :disabled="!nameValue.trim() || !commandValue.trim()"
          >
            Save
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
