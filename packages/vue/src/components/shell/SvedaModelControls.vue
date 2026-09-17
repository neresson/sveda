<script setup>
  import SvedaModelSelect from './SvedaModelSelect.vue';
  import { Switch } from '../../ui/switch';
  import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/tooltip';
  import { Brain } from 'lucide-vue-next';
  import { computed } from 'vue';

  const chatModel = defineModel({ type: String, required: true });
  const thinkingEnabled = defineModel('thinkingEnabled', { type: Boolean, default: true });

  const props = defineProps({
    placeholder: { type: String, required: true },
    thinkingTooltip: { type: String, required: true },
    models: { type: Array, default: () => [] },
  });

  const selectedSupportsThinking = computed(
    () => props.models.some(option => option.id === chatModel.value && option.supportsThinking)
  );
</script>

<template>
  <div class="flex items-center gap-1">
    <SvedaModelSelect
      v-model="chatModel"
      :placeholder="placeholder"
      :models="models"
    />
    <TooltipProvider v-if="selectedSupportsThinking">
      <Tooltip>
        <TooltipTrigger as-child>
          <div class="flex cursor-default items-center gap-1 rounded-md px-0.5 py-0.5 text-muted-foreground hover:bg-muted/50 hover:text-foreground">
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
