<script setup lang="ts">
import { ref } from "vue";
import { useConfigStore } from "../../stores/config";
import { useUiStore } from "../../stores/ui";
import GroupItem from "./GroupItem.vue";
import EditDialog from "../shared/EditDialog.vue";

const config = useConfigStore();
const ui = useUiStore();
const showNewGroupDialog = ref(false);

async function handleCreateGroup(name: string, directory?: string) {
  await config.createGroup(name, directory ?? ".");
  showNewGroupDialog.value = false;
}
</script>

<template>
  <div class="bg-gray-800 border-r border-gray-700 flex flex-col h-full flex-shrink-0">
    <div class="p-3 border-b border-gray-700 flex items-center justify-between">
      <h1 class="text-sm font-bold text-gray-200 tracking-wide uppercase">
        OpenRunner
      </h1>
      <div class="flex items-center gap-1">
        <button
          class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
          title="Settings"
          @click="ui.showSettings()"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
            />
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            />
          </svg>
        </button>
        <button
          class="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 transition-colors"
          title="New Group"
          @click="showNewGroupDialog = true"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 4v16m8-8H4"
            />
          </svg>
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto p-2 space-y-1">
      <GroupItem
        v-for="group in config.groups"
        :key="group.id"
        :group="group"
      />
      <div
        v-if="config.groups.length === 0"
        class="text-center text-gray-500 text-xs mt-8 px-4"
      >
        No groups yet. Click + to create one.
      </div>
    </div>

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
