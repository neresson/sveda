import { computed, onUnmounted, ref } from 'vue';

export type SvedaAgentTaskItem = {
  id: string;
  type: string;
  label: string;
  status: string;
  summary?: string;
  error?: string;
};

export type SvedaAgentTasksState = {
  runId: string | null;
  phase: string;
  tasks: SvedaAgentTaskItem[];
};

export type SvedaAgentTasksPayload = {
  runId?: string;
  phase?: string;
  tasks?: Array<{
    id?: unknown;
    type?: unknown;
    label?: unknown;
    status?: unknown;
    summary?: unknown;
    detail?: unknown;
    error?: unknown;
  }>;
};

const emptyState = (): SvedaAgentTasksState => ({
  runId: null,
  phase: '',
  tasks: [],
});

export function useSvedaAgentTasks(
  options: {
    subscribe?: (
      onPayload: (payload: SvedaAgentTasksPayload) => void
    ) => void | (() => void);
  } = {}
) {
  const state = ref<SvedaAgentTasksState>(emptyState());

  const hasActiveTasks = computed(() =>
    state.value.tasks.some(task => task.status === 'running' || task.status === 'pending')
  );

  const mergePayload = (payload: SvedaAgentTasksPayload) => {
    if (!payload || !Array.isArray(payload.tasks)) {
      return;
    }

    state.value = {
      runId: payload.runId ?? state.value.runId,
      phase: payload.phase ?? state.value.phase,
      tasks: payload.tasks.map(task => ({
        id: String(task.id ?? ''),
        type: String(task.type ?? task.label ?? ''),
        label: String(task.label ?? task.type ?? ''),
        status: String(task.status ?? 'pending'),
        summary:
          task.summary !== undefined && task.summary !== null
            ? String(task.summary)
            : task.detail !== undefined && task.detail !== null
              ? String(task.detail)
              : undefined,
        error: task.error ? String(task.error) : undefined,
      })),
    };
  };

  const clearTasks = () => {
    state.value = emptyState();
  };

  const stopSubscribe = options.subscribe?.(mergePayload) ?? null;

  onUnmounted(() => {
    if (typeof stopSubscribe === 'function') {
      stopSubscribe();
    }
  });

  return {
    agentTasks: state,
    hasActiveAgentTasks: hasActiveTasks,
    clearAgentTasks: clearTasks,
    mergeAgentTasksPayload: mergePayload,
  };
}
