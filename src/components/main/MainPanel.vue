<script setup lang="ts">
import { useUiStore } from "../../stores/ui";
import ProjectDetail from "./ProjectDetail.vue";
import GroupMonitor from "./GroupMonitor.vue";
import SessionDetail from "./SessionDetail.vue";
import SettingsPage from "./SettingsPage.vue";
import EmptyState from "../shared/EmptyState.vue";

const ui = useUiStore();
</script>

<template>
  <div class="flex-1 flex flex-col h-full min-h-0 bg-gray-900">
    <SettingsPage v-if="ui.viewMode === 'settings'" />
    <GroupMonitor
      v-else-if="ui.viewMode === 'groupMonitor' && ui.selectedMonitorGroupId"
      :key="ui.selectedMonitorGroupId"
      :group-id="ui.selectedMonitorGroupId"
    />
    <SessionDetail
      v-else-if="ui.viewMode === 'sessionDetail' && ui.selectedSessionId"
      :key="ui.selectedSessionId"
      :session-id="ui.selectedSessionId"
    />
    <ProjectDetail
      v-else-if="ui.selectedProject && ui.selectedGroup"
      :key="ui.selectedProjectId!"
      :project="ui.selectedProject"
      :group="ui.selectedGroup"
    />
    <EmptyState v-else />
  </div>
</template>
