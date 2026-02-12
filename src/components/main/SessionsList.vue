<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useSessionsStore } from "../../stores/sessions";
import { useUiStore } from "../../stores/ui";
import type { SessionWithStats } from "../../types";
import { formatBytes } from "../../utils/formatters";
import ConfirmDialog from "../shared/ConfirmDialog.vue";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { ArrowLeftIcon, TrashIcon } from "@radix-icons/vue";

const props = defineProps<{
  projectId: string;
  projectName: string;
  groupId: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

const sessions = useSessionsStore();
const ui = useUiStore();
const deleteSessionId = ref<string | null>(null);

onMounted(() => {
  sessions.loadSessionsWithStats(props.groupId, props.projectId);
});

function formatDate(ts: number): string {
  return new Date(ts).toLocaleString();
}

function formatDuration(session: SessionWithStats): string {
  if (!session.endedAt) return "Running";
  const ms = session.endedAt - session.startedAt;
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const secs = seconds % 60;
  if (minutes < 60) return `${minutes}m ${secs}s`;
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  return `${hours}h ${mins}m`;
}

function statusVariant(session: SessionWithStats): "default" | "destructive" | "secondary" {
  if (!session.endedAt) return "default";
  if (session.exitStatus === "errored") return "destructive";
  return "secondary";
}

function statusLabel(session: SessionWithStats): string {
  if (!session.endedAt) return "Running";
  if (session.exitStatus === "errored") return "Errored";
  return "Stopped";
}

function viewSession(sessionId: string) {
  ui.showSessionDetail(sessionId, props.groupId);
}

async function handleDeleteSession() {
  if (deleteSessionId.value) {
    await sessions.deleteSession(props.groupId, deleteSessionId.value);
    deleteSessionId.value = null;
    sessions.loadSessionsWithStats(props.groupId, props.projectId);
  }
}
</script>

<template>
  <div class="flex-1 flex flex-col h-full min-h-0">
    <div class="p-4 border-b border-border flex items-center gap-3">
      <Button variant="ghost" size="icon" class="h-8 w-8" @click="emit('close')">
        <ArrowLeftIcon class="h-4 w-4" />
      </Button>
      <h2 class="text-lg font-semibold text-foreground">
        Sessions - {{ props.projectName }}
      </h2>
    </div>

    <div class="flex-1 overflow-y-auto p-4">
      <div v-if="sessions.loading" class="text-center text-muted-foreground py-8">
        Loading sessions...
      </div>
      <div v-else-if="sessions.sessionsWithStats.length === 0" class="text-center text-muted-foreground py-8">
        No sessions recorded yet. Start the process to create a session.
      </div>
      <div v-else class="space-y-2">
        <Card
          v-for="session in sessions.sessionsWithStats"
          :key="session.id"
          class="cursor-pointer hover:border-primary/50 transition-colors"
          @click="viewSession(session.id)"
        >
          <CardContent class="p-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <span class="text-sm text-foreground">{{ formatDate(session.startedAt) }}</span>
                <span :class="statusVariant(session) === 'default' ? 'text-green-400' : statusVariant(session) === 'destructive' ? 'text-destructive' : 'text-muted-foreground'" class="text-xs font-medium">
                  {{ statusLabel(session) }}
                </span>
              </div>
              <div class="flex items-center gap-3">
                <span class="text-xs text-muted-foreground">{{ formatDuration(session) }}</span>
                <Button
                  variant="ghost"
                  size="icon"
                  class="h-6 w-6 text-muted-foreground hover:text-destructive"
                  title="Delete session"
                  @click.stop="deleteSessionId = session.id"
                >
                  <TrashIcon class="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
            <div class="flex items-center gap-3 mt-1.5 text-[11px] text-muted-foreground">
              <span>{{ session.logCount }} logs</span>
              <span>{{ formatBytes(session.logSize) }}</span>
              <span>{{ session.metricCount }} metrics</span>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>

    <ConfirmDialog
      :open="!!deleteSessionId"
      title="Delete Session"
      message="Are you sure you want to delete this session and all its logs?"
      @confirm="handleDeleteSession"
      @cancel="deleteSessionId = null"
    />
  </div>
</template>
