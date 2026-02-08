import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { useConfigStore } from "./config";

export type ViewMode = "project" | "groupMonitor" | "settings" | "sessionDetail";

export const useUiStore = defineStore("ui", () => {
  const selectedGroupId = ref<string | null>(null);
  const selectedProjectId = ref<string | null>(null);
  const expandedGroups = ref<Set<string>>(new Set());
  const viewMode = ref<ViewMode>("project");
  const selectedMonitorGroupId = ref<string | null>(null);
  const selectedSessionId = ref<string | null>(null);
  const showMonitor = ref(false);

  const configStore = useConfigStore();

  const selectedGroup = computed(
    () =>
      configStore.groups.find((g) => g.id === selectedGroupId.value) ?? null,
  );

  const selectedProject = computed(() => {
    if (!selectedGroup.value || !selectedProjectId.value) return null;
    return (
      selectedGroup.value.projects.find(
        (p) => p.id === selectedProjectId.value,
      ) ?? null
    );
  });

  function selectProject(groupId: string, projectId: string) {
    selectedGroupId.value = groupId;
    selectedProjectId.value = projectId;
    expandedGroups.value.add(groupId);
    viewMode.value = "project";
  }

  function toggleGroup(groupId: string) {
    if (expandedGroups.value.has(groupId)) {
      expandedGroups.value.delete(groupId);
    } else {
      expandedGroups.value.add(groupId);
    }
  }

  function isGroupExpanded(groupId: string) {
    return expandedGroups.value.has(groupId);
  }

  function clearSelection() {
    selectedGroupId.value = null;
    selectedProjectId.value = null;
    viewMode.value = "project";
  }

  function showGroupMonitor(groupId: string) {
    selectedMonitorGroupId.value = groupId;
    viewMode.value = "groupMonitor";
  }

  function showSettings() {
    viewMode.value = "settings";
  }

  function showSessionDetail(sessionId: string) {
    selectedSessionId.value = sessionId;
    viewMode.value = "sessionDetail";
  }

  function backToProject() {
    viewMode.value = "project";
    selectedSessionId.value = null;
  }

  return {
    selectedGroupId,
    selectedProjectId,
    selectedGroup,
    selectedProject,
    expandedGroups,
    viewMode,
    selectedMonitorGroupId,
    selectedSessionId,
    showMonitor,
    selectProject,
    toggleGroup,
    isGroupExpanded,
    clearSelection,
    showGroupMonitor,
    showSettings,
    showSessionDetail,
    backToProject,
  };
});
