<script setup>
  import { Badge } from '../../ui/badge';
  import { Card, CardContent, CardHeader, CardTitle } from '../../ui/card';
  import { CheckCircle2, CircleDashed, Loader2, Search, XCircle } from 'lucide-vue-next';
  import { computed } from 'vue';
  import { useSvedaT } from '../../i18n/index';

  const props = defineProps({
    phase: {
      type: String,
      default: '',
    },
    tasks: {
      type: Array,
      default: () => [],
    },
    showLiveStatus: {
      type: Boolean,
      default: false,
    },
  });

  const t = useSvedaT();

  const visible = computed(() => Array.isArray(props.tasks) && props.tasks.length > 0);

  const phaseLabel = computed(() => {
    if (props.phase === 'preflight') {
      return t('phaseSearchSubagents');
    }
    if (props.phase === 'spawn') {
      return t('agentTasksSpawnPhase');
    }
    if (props.phase === 'tool') {
      return t('phaseToolCalling');
    }
    return t('agentTasksTitle');
  });

  const labelForTask = task => {
    const type = String(task.type || '');
    if (type) {
      const key = `agentTaskType_${type}`;
      const translated = t(key);
      if (translated !== key) {
        return translated;
      }
    }
    return task.label || task.type;
  };

  const statusLabel = status => {
    const key = `agentTaskStatus${String(status).charAt(0).toUpperCase()}${String(status).slice(1)}`;
    const translated = t(key);
    return translated !== key ? translated : status;
  };

  const isLiveTaskStatus = status =>
    props.showLiveStatus && (status === 'running' || status === 'pending');

  const statusIcon = status => {
    if (isLiveTaskStatus(status)) {
      return Loader2;
    }
    if (status === 'error' || status === 'failed') {
      return XCircle;
    }
    if (status === 'success' || status === 'completed') {
      return CheckCircle2;
    }
    return CircleDashed;
  };

  const statusVariant = status => {
    if (status === 'error' || status === 'failed') {
      return 'destructive';
    }
    if (status === 'success' || status === 'completed') {
      return 'default';
    }
    return 'secondary';
  };

  const taskRowClass = status =>
    [
      'flex items-start gap-3 rounded-md border border-border/60 bg-background/80 px-3 py-2',
      isLiveTaskStatus(status) ? 'border-sky-300/70' : '',
    ].join(' ');

  const statusIconClass = status =>
    [
      'mt-0.5 h-4 w-4 shrink-0',
      isLiveTaskStatus(status) ? 'animate-spin text-sky-600' : '',
      status === 'success' || status === 'completed' ? 'text-emerald-600' : '',
      status === 'error' || status === 'failed' ? 'text-destructive' : '',
    ].join(' ');
</script>

<template>
  <Card
    v-if="visible"
    class="mb-3 border-sky-200/80 bg-sky-50/50 dark:border-sky-900/60 dark:bg-sky-950/30"
  >
    <CardHeader class="flex flex-row items-center gap-2 space-y-0 px-4 py-3">
      <Search class="h-4 w-4 shrink-0 text-sky-700 dark:text-sky-300" />
      <CardTitle class="text-sm font-medium text-sky-900 dark:text-sky-100">
        {{ phaseLabel }}
      </CardTitle>
    </CardHeader>
    <CardContent class="space-y-2 px-4 pb-3 pt-0">
      <div
        v-for="task in tasks"
        :key="task.id"
        :class="taskRowClass(task.status)"
      >
        <component
          :is="statusIcon(task.status)"
          :class="statusIconClass(task.status)"
        />
        <div class="min-w-0 flex-1">
          <div class="flex flex-wrap items-center gap-2">
            <span class="text-sm font-medium text-foreground">{{ labelForTask(task) }}</span>
            <Badge
              :variant="statusVariant(task.status)"
              class="text-xs"
            >
              {{ statusLabel(task.status) }}
            </Badge>
          </div>
          <p
            v-if="task.summary"
            class="mt-1 line-clamp-2 text-xs text-muted-foreground"
          >
            {{ task.summary }}
          </p>
          <p
            v-else-if="task.error"
            class="mt-1 line-clamp-2 text-xs text-destructive"
          >
            {{ task.error }}
          </p>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
