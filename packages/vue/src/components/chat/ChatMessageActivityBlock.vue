<script setup>
  import {
    getActivityGroupKey,
    getActivityGroupLabel,
    getToolNameFromPart,
    parseToolResultOutput,
  } from '../../lib/assistantMessage.js';
  import { useVedaT } from '../../i18n/index.js';
  import { CheckCircle, ChevronDown, ChevronRight, CircleStop, ExternalLink, Search, Terminal, XCircle } from 'lucide-vue-next';
  import { computed, ref, watch } from 'vue';
  import AnimatedStreamBlock from './AnimatedStreamBlock.vue';

  const props = defineProps({
    invocations: {
      type: Array,
      default: () => [],
    },
    legacyActivity: {
      type: Object,
      default: null,
    },
    contentAnimationsEnabled: {
      type: Boolean,
      default: false,
    },
  });

  const emit = defineEmits(['activity-link-click']);

  const t = useVedaT();

  const translateToolName = toolName => {
    if (!toolName) {
      return '';
    }
    const key = `tools.${toolName}`;
    const translated = t(key);
    return translated === key ? toolName : translated;
  };

  const isToolInvocationComplete = inv =>
    inv?.state === 'result' || inv?.state === 'output-available' || inv?.state === 'output-error' || inv?.state === 'output-denied';

  const getToolInvocationStatus = inv => {
    if (!isToolInvocationComplete(inv)) {
      if (!props.contentAnimationsEnabled) {
        return 'success';
      }
      return 'running';
    }
    if (inv.state === 'output-error' || inv.state === 'output-denied') {
      return inv.state === 'output-denied' ? 'stopped' : 'error';
    }

    const resultData = parseToolResultOutput(inv.output !== undefined ? inv.output : inv.result);
    if (resultData && (resultData.error || resultData.success === false)) {
      return 'error';
    }

    return 'success';
  };

  const toolSteps = computed(() => {
    if (props.invocations.length > 0) {
      return props.invocations.map((inv, index) => {
        const isComplete = isToolInvocationComplete(inv);
        const resultData = parseToolResultOutput(inv.output !== undefined ? inv.output : inv.result);
        const status = getToolInvocationStatus(inv);
        const toolName = getToolNameFromPart(inv);

        let detail = '';
        if (isComplete) {
          if (resultData) {
            const data = resultData.data;
            if (data && typeof data === 'object' && data !== null && 'content' in data && data.content) {
              detail = String(data.content);
            } else if (resultData.error) {
              detail = String(resultData.error);
            } else if (resultData.message) {
              detail = String(resultData.message);
            } else {
              detail = JSON.stringify(resultData, null, 2);
            }
          } else if (inv.errorText) {
            detail = String(inv.errorText);
          } else {
            detail = String(inv.output !== undefined ? inv.output : (inv.result ?? ''));
          }
        } else {
          detail = JSON.stringify(inv.args !== undefined ? inv.args : inv.input, null, 2);
        }

        if (detail.length > 2000) {
          detail = detail.slice(0, 2000) + '...';
        }

        const translatedName = translateToolName(toolName);
        let label = translatedName;
        const display = resultData?.display;
        if (isComplete && display && typeof display === 'object' && display !== null && 'name' in display && display.name) {
          label = `${translatedName}: ${display.name}`;
        }

        const activityGroupKey = getActivityGroupKey(toolName);
        const groupPayload = activityGroupKey
          ? {
              activityGroupKey,
              activityGroupLabel: getActivityGroupLabel(toolName, translatedName),
            }
          : {};

        return {
          id: inv.toolCallId || `tool-${toolName}-${index}`,
          kind: 'tool',
          toolName,
          label,
          status,
          detail,
          display: display && typeof display === 'object' ? display : undefined,
          ...groupPayload,
        };
      });
    }

    const steps = props.legacyActivity?.steps ?? [];
    return steps.filter(step => step.kind === 'tool' || step.kind === 'error' || step.kind === 'subagent');
  });

  const expandedSteps = ref({});
  const expandedGroups = ref({});
  let prevSteps = [];

  watch(
    () => toolSteps.value,
    steps => {
      const prevMap = new Map(prevSteps.map(step => [step.id, step]));
      steps.forEach(step => {
        if (step.kind === 'tool') {
          if (step.status === 'running') {
            expandedSteps.value[step.id] = true;
          } else if (step.status === 'success' || step.status === 'error' || step.status === 'stopped') {
            const wasRunning = prevMap.get(step.id)?.status === 'running';
            if (wasRunning) {
              expandedSteps.value[step.id] = false;
            }
          }
        }
      });
      prevSteps = [...steps];
    },
    { deep: true, immediate: true }
  );

  const phaseStatusLabel = computed(() => {
    if (!props.contentAnimationsEnabled || props.invocations.length === 0) {
      return '';
    }
    const hasRunning = props.invocations.some(inv => !isToolInvocationComplete(inv));
    if (hasRunning) {
      return t('phaseToolCalling');
    }
    return '';
  });

  const isSubagentStep = step => step.kind === 'subagent';

  const getStepGroupKey = step => {
    if (step.kind !== 'tool') {
      return '';
    }
    return step.activityGroupKey || '';
  };

  const activityItems = computed(() => {
    const items = [];

    for (const step of toolSteps.value) {
      const groupKey = getStepGroupKey(step);
      const previous = items[items.length - 1];
      if (groupKey && previous?.type === 'group' && previous.groupKey === groupKey) {
        previous.steps.push(step);
        continue;
      }

      if (groupKey) {
        items.push({
          type: 'group',
          groupKey,
          label: step.activityGroupLabel || step.label,
          steps: [step],
        });
      } else {
        items.push({
          type: 'step',
          step,
        });
      }
    }

    return items.map(item => {
      if (item.type !== 'group' || item.steps.length === 1) {
        return item.type === 'group' ? { type: 'step', step: item.steps[0] } : item;
      }

      return {
        ...item,
        id: `${item.groupKey}-${item.steps[0].id}`,
      };
    });
  });

  const getActivityItemKey = item => {
    return item.type === 'group' ? item.id : item.step.id;
  };

  const getGroupStatus = group => {
    if (group.steps.some(step => step.status === 'running')) {
      return 'running';
    }
    if (group.steps.some(step => step.status === 'error')) {
      return 'error';
    }
    if (group.steps.some(step => step.status === 'stopped')) {
      return 'stopped';
    }
    return 'success';
  };

  const getGroupLabel = group => {
    return t('activityGroupSteps', {
      label: group.label,
      count: group.steps.length,
    });
  };

  const hasDisplayLink = step => step.display && step.display.link && step.status === 'success';

  const handleStepClick = step => {
    if (hasDisplayLink(step)) {
      emit('activity-link-click', step.display.link);
      return;
    }
    expandedSteps.value[step.id] = !expandedSteps.value[step.id];
  };

  const handleGroupClick = group => {
    expandedGroups.value[group.id] = !expandedGroups.value[group.id];
  };

  const isStepExpanded = step => !!expandedSteps.value[step.id];
  const isGroupExpanded = group => !!expandedGroups.value[group.id];
</script>

<template>
  <div
    v-if="toolSteps.length > 0"
    class="space-y-0.5"
  >
    <p
      v-if="phaseStatusLabel"
      class="text-xs text-muted-foreground/80"
    >
      {{ phaseStatusLabel }}
    </p>
    <template
      v-for="item in activityItems"
      :key="getActivityItemKey(item)"
    >
      <div v-if="item.type === 'group'">
        <button
          type="button"
          class="flex w-full items-center gap-1.5 py-0.5 text-left text-muted-foreground/90 hover:text-muted-foreground"
          @click="handleGroupClick(item)"
        >
          <CheckCircle
            v-if="getGroupStatus(item) === 'success'"
            class="h-3 w-3 text-green-500"
          />
          <XCircle
            v-else-if="getGroupStatus(item) === 'error'"
            class="h-3 w-3 text-red-500"
          />
          <CircleStop
            v-else-if="getGroupStatus(item) === 'stopped'"
            class="h-3 w-3 text-muted-foreground"
          />
          <Terminal
            v-else
            class="h-3 w-3 text-muted-foreground"
          />
          <span
            class="min-w-0 flex-1 truncate text-xs"
            :class="getGroupStatus(item) === 'error' ? 'text-red-500/90' : ''"
          >
            {{ getGroupLabel(item) }}
          </span>
          <component
            :is="isGroupExpanded(item) ? ChevronDown : ChevronRight"
            class="ml-auto h-3 w-3 text-muted-foreground"
          />
        </button>
        <div
          v-if="isGroupExpanded(item)"
          class="space-y-0.5 pl-4"
        >
          <div
            v-for="step in item.steps"
            :key="step.id"
          >
            <button
              type="button"
              class="flex w-full items-center gap-1.5 py-0.5 text-left text-muted-foreground/90"
              :class="{ 'cursor-pointer hover:text-muted-foreground': hasDisplayLink(step) }"
              @click="handleStepClick(step)"
            >
              <CheckCircle
                v-if="step.status === 'success'"
                class="h-3 w-3 text-green-500"
              />
              <XCircle
                v-else-if="step.status === 'error'"
                class="h-3 w-3 text-red-500"
              />
              <CircleStop
                v-else-if="step.status === 'stopped'"
                class="h-3 w-3 text-muted-foreground"
              />
              <Search
                v-else-if="isSubagentStep(step)"
                class="h-3 w-3 text-sky-600"
              />
              <Terminal
                v-else
                class="h-3 w-3 text-muted-foreground"
              />
              <span
                class="min-w-0 flex-1 text-xs"
                :class="[
                  step.status === 'error' ? 'text-red-500/90' : '',
                  isSubagentStep(step) ? 'text-sky-700/80 dark:text-sky-400/80' : '',
                  hasDisplayLink(step) ? 'underline decoration-dotted underline-offset-2 hover:text-primary' : '',
                ]"
              >
                {{ step.label }}
              </span>
              <ExternalLink
                v-if="hasDisplayLink(step)"
                class="h-3 w-3 flex-shrink-0 text-muted-foreground"
              />
              <component
                v-else-if="step.detail || step.kind === 'tool' || step.kind === 'subagent'"
                :is="isStepExpanded(step) ? ChevronDown : ChevronRight"
                class="ml-auto h-3 w-3 text-muted-foreground"
              />
            </button>
            <div
              v-if="isStepExpanded(step) && !hasDisplayLink(step) && (step.detail || step.kind === 'tool' || step.kind === 'subagent')"
              class="pb-1 pl-4 text-muted-foreground/80"
            >
              <AnimatedStreamBlock
                v-if="step.kind === 'tool'"
                :text="step.detail || ''"
                :animated="contentAnimationsEnabled"
                max-height="8rem"
                :show-mask="true"
                variant="mono"
              />
              <div
                v-else
                :class="['max-h-32 overflow-y-auto whitespace-pre-wrap break-words text-xs', step.kind === 'thinking' ? 'font-mono' : '']"
              >
                {{ step.detail }}
              </div>
            </div>
          </div>
        </div>
      </div>
      <div v-else>
        <button
          type="button"
          class="flex w-full items-center gap-1.5 py-0.5 text-left text-muted-foreground/90"
          :class="{ 'cursor-pointer hover:text-muted-foreground': hasDisplayLink(item.step) }"
          @click="handleStepClick(item.step)"
        >
          <CheckCircle
            v-if="item.step.status === 'success'"
            class="h-3 w-3 text-green-500"
          />
          <XCircle
            v-else-if="item.step.status === 'error'"
            class="h-3 w-3 text-red-500"
          />
          <CircleStop
            v-else-if="item.step.status === 'stopped'"
            class="h-3 w-3 text-muted-foreground"
          />
          <Search
            v-else-if="isSubagentStep(item.step)"
            class="h-3 w-3 text-sky-600"
          />
          <Terminal
            v-else
            class="h-3 w-3 text-muted-foreground"
          />
          <span
            class="min-w-0 flex-1 text-xs"
            :class="[
              item.step.status === 'error' ? 'text-red-500/90' : '',
              isSubagentStep(item.step) ? 'text-sky-700/80 dark:text-sky-400/80' : '',
              hasDisplayLink(item.step) ? 'underline decoration-dotted underline-offset-2 hover:text-primary' : '',
            ]"
          >
            {{ item.step.label }}
          </span>
          <ExternalLink
            v-if="hasDisplayLink(item.step)"
            class="h-3 w-3 flex-shrink-0 text-muted-foreground"
          />
          <component
            v-else-if="item.step.detail || item.step.kind === 'tool' || item.step.kind === 'subagent'"
            :is="isStepExpanded(item.step) ? ChevronDown : ChevronRight"
            class="ml-auto h-3 w-3 text-muted-foreground"
          />
        </button>
        <div
          v-if="isStepExpanded(item.step) && !hasDisplayLink(item.step) && (item.step.detail || item.step.kind === 'tool' || item.step.kind === 'subagent')"
          class="pb-1 pl-4 text-muted-foreground/80"
        >
          <AnimatedStreamBlock
            v-if="item.step.kind === 'tool'"
            :text="item.step.detail || ''"
            :animated="contentAnimationsEnabled"
            max-height="8rem"
            :show-mask="true"
            variant="mono"
          />
          <div
            v-else
            :class="['max-h-32 overflow-y-auto whitespace-pre-wrap break-words text-xs', item.step.kind === 'thinking' ? 'font-mono' : '']"
          >
            {{ item.step.detail }}
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
