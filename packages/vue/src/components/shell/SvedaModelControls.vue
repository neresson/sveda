<script setup>
  import SvedaModelSelect from './SvedaModelSelect.vue';
  import { Switch } from '../../ui/switch';
  import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/tooltip';
  import { Brain } from 'lucide-vue-next';
  import { computed } from 'vue';
  import { useSvedaChrome } from '../../appearance';

  const chatModel = defineModel({ type: String, required: true });
  const thinkingEnabled = defineModel('thinkingEnabled', { type: Boolean, default: true });

  const props = defineProps({
    placeholder: { type: String, required: true },
    thinkingTooltip: { type: String, required: true },
    models: { type: Array, default: () => [] },
  });

  const chrome = useSvedaChrome();

  const selectedSupportsThinking = computed(
    () => props.models.some(option => option.id === chatModel.value && option.supportsThinking)
  );

  const showModelSelect = computed(() => chrome.modelSelect && props.models.length > 0);
  const showThinking = computed(() => chrome.thinking && selectedSupportsThinking.value);
</script>

<template>
  <div
    v-if="showModelSelect || showThinking"
    class="flex items-center gap-1"
  >
    <SvedaModelSelect
      v-if="showModelSelect"
      v-model="chatModel"
      :placeholder="placeholder"
      :models="models"
    />
    <TooltipProvider v-if="showThinking">
      <Tooltip>
        <TooltipTrigger as-child>
          <div class="flex cursor-default items-center gap-1 px-0.5 py-0.5 text-muted-foreground hover:text-foreground">
            <Brain class="h-3.5 w-3.5 shrink-0" />
            <Switch
              v-model="thinkingEnabled"
              size="sm"
              :aria-label="thinkingTooltip"
            />
          </div>
        </TooltipTrigger>
        <TooltipContent class="max-w-xs">
          {{ thinkingTooltip }}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  </div>
</template>
