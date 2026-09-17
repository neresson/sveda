<script setup>
  import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/tooltip';

  defineProps({
    percent: { type: Number, required: true },
    tooltip: { type: String, required: true },
  });
</script>

<template>
  <div class="flex w-full items-center px-1 pb-1 pt-1">
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger as-child>
          <div class="h-[2px] w-full cursor-help overflow-hidden rounded-full bg-primary/20">
            <div
              class="h-full bg-primary transition-all duration-300"
              :style="{ width: `${percent}%` }"
            />
          </div>
        </TooltipTrigger>
        <TooltipContent
          side="top"
          align="end"
        >
          <div class="text-xs">
            {{ tooltip }}
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  </div>
</template>
