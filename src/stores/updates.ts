import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke, listen } from "@/lib/api";

export interface UpdateInfo {
  version: string;
  releaseDate: string | null;
  releaseNotes: string | null;
}

export interface DownloadProgress {
  percent: number;
  bytesPerSecond: number;
  transferred: number;
  total: number;
}

export const useUpdatesStore = defineStore("updates", () => {
  // State
  const currentVersion = ref<string>("");
  const checking = ref(false);
  const available = ref(false);
  const downloading = ref(false);
  const downloaded = ref(false);
  const progress = ref(0);
  const error = ref<string | null>(null);
  const updateVersion = ref<string | null>(null);
  const releaseNotes = ref<string | null>(null);
  const releaseDate = ref<string | null>(null);
  const autoUpdateSupported = ref(true);
  const initialized = ref(false);
  const isDevMode = ref(false);

  // Computed
  const hasUpdate = computed(() => available.value || downloaded.value);
  const updateReady = computed(() => downloaded.value);
  const isChecking = computed(() => checking.value);
  const isDownloading = computed(() => downloading.value);

  // Actions
  async function initialize(): Promise<void> {
    if (initialized.value) return;

    try {
      // Get initial version info
      const result = await invoke<{
        version: string;
        updateState: {
          checking: boolean;
          available: boolean;
          downloading: boolean;
          progress: number;
          downloaded: boolean;
          error: string | null;
          version: string | null;
          releaseNotes: string | null;
          releaseDate: string | null;
        };
        autoUpdateSupported: boolean;
      }>("get-app-version");

      currentVersion.value = result.version;
      autoUpdateSupported.value = result.autoUpdateSupported;

      // Check if running in dev mode
      isDevMode.value = await invoke<boolean>("is-dev-mode");

      // Restore state if there's an active update
      if (result.updateState) {
        checking.value = result.updateState.checking;
        available.value = result.updateState.available;
        downloading.value = result.updateState.downloading;
        progress.value = result.updateState.progress;
        downloaded.value = result.updateState.downloaded;
        error.value = result.updateState.error;
        updateVersion.value = result.updateState.version;
        releaseNotes.value = result.updateState.releaseNotes;
        releaseDate.value = result.updateState.releaseDate;
      }

      // Set up event listeners
      setupEventListeners();
      initialized.value = true;
    } catch (e) {
      console.error("[UpdatesStore] Failed to initialize:", e);
    }
  }

  function setupEventListeners(): void {
    // Checking for updates
    listen("update-checking", () => {
      checking.value = true;
      error.value = null;
    });

    // Update available
    listen<UpdateInfo>("update-available", (data) => {
      checking.value = false;
      available.value = true;
      updateVersion.value = data.version;
      releaseNotes.value = data.releaseNotes;
      releaseDate.value = data.releaseDate;
    });

    // No update available
    listen("update-not-available", () => {
      checking.value = false;
      available.value = false;
    });

    // Download progress
    listen<DownloadProgress>("update-download-progress", (data) => {
      downloading.value = true;
      progress.value = data.percent;
    });

    // Update downloaded
    listen<{ version: string; releaseNotes: string | null }>(
      "update-downloaded",
      (data) => {
        downloading.value = false;
        downloaded.value = true;
        progress.value = 100;
        updateVersion.value = data.version;
        releaseNotes.value = data.releaseNotes;
      }
    );

    // Error
    listen<{ message: string }>("update-error", (data) => {
      checking.value = false;
      downloading.value = false;
      error.value = data.message;
    });
  }

  async function checkForUpdates(): Promise<void> {
    if (checking.value) return;

    checking.value = true;
    error.value = null;

    try {
      const result = await invoke<{
        checking: boolean;
        available: boolean;
        downloading: boolean;
        progress: number;
        downloaded: boolean;
        error: string | null;
        version: string | null;
        releaseNotes: string | null;
        releaseDate: string | null;
        currentVersion: string;
        autoUpdateSupported: boolean;
      }>("check-for-updates");

      currentVersion.value = result.currentVersion;
      autoUpdateSupported.value = result.autoUpdateSupported;
    } catch (e) {
      console.error("[UpdatesStore] Check for updates failed:", e);
      checking.value = false;
      error.value = e instanceof Error ? e.message : "Failed to check for updates";
    }
  }

  async function downloadUpdate(): Promise<void> {
    if (!available.value || downloading.value) return;

    downloading.value = true;
    progress.value = 0;
    error.value = null;

    try {
      const result = await invoke<{ success?: boolean; openedReleasePage?: boolean }>(
        "download-update"
      );

      // If we opened the release page (macOS without code signing),
      // reset the downloading state
      if (result.openedReleasePage) {
        downloading.value = false;
      }
    } catch (e) {
      console.error("[UpdatesStore] Download update failed:", e);
      downloading.value = false;
      error.value = e instanceof Error ? e.message : "Failed to download update";
    }
  }

  async function installUpdate(): Promise<void> {
    if (!downloaded.value) return;

    try {
      await invoke("install-update");
      // App will quit and restart, no need to handle response
    } catch (e) {
      console.error("[UpdatesStore] Install update failed:", e);
      error.value = e instanceof Error ? e.message : "Failed to install update";
    }
  }

  function clearError(): void {
    error.value = null;
  }

  /**
   * Set mock update data for testing the update preview UI in dev mode.
   * This allows developers to see how release notes will render.
   */
  function setMockUpdate(notes: string): void {
    if (!isDevMode.value) return;

    // Reset any existing state
    checking.value = false;
    downloading.value = false;
    downloaded.value = false;
    error.value = null;

    // Set mock update data
    available.value = true;
    updateVersion.value = "99.0.0-preview";
    releaseDate.value = new Date().toISOString();
    releaseNotes.value = notes;
  }

  /**
   * Clear mock update data
   */
  function clearMockUpdate(): void {
    if (!isDevMode.value) return;

    available.value = false;
    downloading.value = false;
    downloaded.value = false;
    progress.value = 0;
    updateVersion.value = null;
    releaseNotes.value = null;
    releaseDate.value = null;
    error.value = null;
  }

  return {
    // State
    currentVersion,
    checking,
    available,
    downloading,
    downloaded,
    progress,
    error,
    updateVersion,
    releaseNotes,
    releaseDate,
    autoUpdateSupported,
    isDevMode,

    // Computed
    hasUpdate,
    updateReady,
    isChecking,
    isDownloading,

    // Actions
    initialize,
    checkForUpdates,
    downloadUpdate,
    installUpdate,
    clearError,
    setMockUpdate,
    clearMockUpdate,
  };
});
