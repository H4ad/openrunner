<script setup lang="ts">
import { ref } from "vue";
import { useConfigStore } from "../../stores/config";
import { useUiStore } from "../../stores/ui";
import { useUpdatesStore } from "../../stores/updates";
import GroupItem from "./GroupItem.vue";
import EditDialog from "../shared/EditDialog.vue";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import { GearIcon, PlusIcon, UploadIcon, HomeIcon } from "@radix-icons/vue";
import { open } from "@/lib/dialog";

const config = useConfigStore();
const ui = useUiStore();
const updatesStore = useUpdatesStore();
const showNewGroupDialog = ref(false);

async function handleCreateGroup(name: string, directory?: string) {
  await config.createGroup(name, directory ?? ".");
  showNewGroupDialog.value = false;
}

async function importGroup() {
  const filePath = await open({
    filters: [
      { name: "YAML", extensions: ["yaml", "yml"] },
      { name: "All Files", extensions: ["*"] },
    ],
    multiple: false,
  });
  if (filePath) {
    try {
      await config.importGroup(filePath as string);
    } catch (e) {
      console.error("Failed to import group:", e);
    }
  }
}
</script>

<template>
  <div class="bg-card border-r border-border flex flex-col h-full flex-shrink-0">
    <div class="p-3 border-b border-border flex items-center justify-between">
      <Button
        variant="ghost"
        size="icon"
        class="h-7 w-7"
        title="Home"
        @click="ui.showHome()"
      >
        <HomeIcon class="h-4 w-4" />
      </Button>
      <div class="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          class="h-7 w-7"
          title="Import Group"
          @click="importGroup"
        >
          <UploadIcon class="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          class="h-7 w-7"
          title="New Group"
          @click="showNewGroupDialog = true"
        >
          <PlusIcon class="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          class="h-7 w-7 relative"
          title="Settings"
          @click="ui.showSettings()"
        >
          <GearIcon class="h-4 w-4" />
          <!-- Update badge -->
          <span
            v-if="updatesStore.hasUpdate"
            class="absolute -top-0.5 -right-0.5 h-2.5 w-2.5 rounded-full bg-blue-500 border border-card"
            :class="{ 'animate-pulse': updatesStore.downloaded }"
          />
        </Button>
      </div>
    </div>

    <ScrollArea class="flex-1 p-2">
      <div class="space-y-1">
        <GroupItem
          v-for="group in config.groups"
          :key="group.id"
          :group="group"
        />
        <div
          v-if="config.groups.length === 0"
          class="text-center text-muted-foreground text-xs mt-8 px-4"
        >
          No groups yet. Click + to create one.
        </div>
      </div>
    </ScrollArea>

    <EditDialog
      :open="showNewGroupDialog"
      title="New Group"
      label="Group Name"
      placeholder="My Project Group"
      secondary-label="Working Directory"
      secondary-placeholder="/path/to/workspace"
      secondary-browse-directory
      @confirm="handleCreateGroup"
      @cancel="showNewGroupDialog = false"
    />
  </div>
</template>
