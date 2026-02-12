import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { useConfigStore } from "./config";

export type ViewMode = "home" | "project" | "groupMonitor" | "sessionDetail";

export interface ProjectSelection {
  groupId: string;
  projectId: string;
}

export const useUiStore = defineStore("ui", () => {
  const selectedGroupId = ref<string | null>(null);
  const selectedProjectId = ref<string | null>(null);
  const expandedGroups = ref<Set<string>>(new Set());
  const viewMode = ref<ViewMode>("home");
  const selectedMonitorGroupId = ref<string | null>(null);
  const selectedSessionId = ref<string | null>(null);
  const selectedSessionGroupId = ref<string | null>(null);
  const showMonitor = ref(false);
  const selectedProjects = ref<ProjectSelection[]>([]);
  const lastSelectedProject = ref<ProjectSelection | null>(null);

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

  function selectProject(groupId: string, projectId: string, shiftKey = false) {
    selectedGroupId.value = groupId;
    selectedProjectId.value = projectId;
    expandedGroups.value.add(groupId);
    viewMode.value = "project";

    if (shiftKey && lastSelectedProject.value && lastSelectedProject.value.groupId === groupId) {
      selectRange(groupId, projectId);
    } else {
      if (!shiftKey) {
        selectedProjects.value = [];
      }
      addToSelection(groupId, projectId);
      lastSelectedProject.value = { groupId, projectId };
    }
  }

  function selectRange(groupId: string, projectId: string) {
    const group = configStore.groups.find((g) => g.id === groupId);
    if (!group || !lastSelectedProject.value) return;

    const projects = group.projects;
    const currentIndex = projects.findIndex((p) => p.id === projectId);
    const lastIndex = projects.findIndex(
      (p) => p.id === lastSelectedProject.value!.projectId,
    );

    if (currentIndex === -1 || lastIndex === -1) return;

    const start = Math.min(currentIndex, lastIndex);
    const end = Math.max(currentIndex, lastIndex);

    const rangeToSelect = projects
      .slice(start, end + 1)
      .map((p) => ({ groupId, projectId: p.id }));

    selectedProjects.value = [...selectedProjects.value, ...rangeToSelect];

    const uniqueSelections = new Map<string, ProjectSelection>();
    for (const sel of selectedProjects.value) {
      uniqueSelections.set(`${sel.groupId}:${sel.projectId}`, sel);
    }
    selectedProjects.value = Array.from(uniqueSelections.values());
  }

  function addToSelection(groupId: string, projectId: string) {
    const key = `${groupId}:${projectId}`;
    const existingKey = selectedProjects.value.find(
      (s) => s.groupId === groupId && s.projectId === projectId,
    );
    if (!existingKey) {
      selectedProjects.value.push({ groupId, projectId });
    }
  }

  function isProjectSelected(groupId: string, projectId: string) {
    return selectedProjects.value.some(
      (s) => s.groupId === groupId && s.projectId === projectId,
    );
  }

  function clearProjectSelection() {
    selectedProjects.value = [];
    lastSelectedProject.value = null;
  }

  function clearMultiSelection() {
    selectedProjects.value = [];
    lastSelectedProject.value = null;
  }

  function getSelectedProjectIds(): string[] {
    return selectedProjects.value.map((s) => s.projectId);
  }

  function toggleGroup(groupId: string) {
    if (expandedGroups.value.has(groupId)) {
      expandedGroups.value.delete(groupId);
    } else {
      expandedGroups.value.add(groupId);
    }
  }

  function selectGroup(groupId: string) {
    selectedGroupId.value = groupId;
    selectedProjectId.value = null;
  }

  function isGroupExpanded(groupId: string) {
    return expandedGroups.value.has(groupId);
  }

  function clearSelection() {
    selectedGroupId.value = null;
    selectedProjectId.value = null;
    viewMode.value = "home";
  }

  function showGroupMonitor(groupId: string) {
    selectedMonitorGroupId.value = groupId;
    viewMode.value = "groupMonitor";
  }

  function showHome() {
    viewMode.value = "home";
  }

  function showSessionDetail(sessionId: string, groupId: string) {
    selectedSessionId.value = sessionId;
    selectedSessionGroupId.value = groupId;
    viewMode.value = "sessionDetail";
  }

  function backToProject() {
    viewMode.value = "project";
    selectedSessionId.value = null;
    selectedSessionGroupId.value = null;
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
    selectedSessionGroupId,
    showMonitor,
    selectedProjects,
    lastSelectedProject,
    selectProject,
    selectRange,
    addToSelection,
    toggleGroup,
    selectGroup,
    isGroupExpanded,
    clearSelection,
    isProjectSelected,
    clearProjectSelection,
    clearMultiSelection,
    getSelectedProjectIds,
    showGroupMonitor,
    showHome,
    showSessionDetail,
    backToProject,
  };
});
