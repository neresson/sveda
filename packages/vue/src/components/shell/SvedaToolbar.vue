<script setup>
  import { Button } from '../../ui/button';
  import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/tooltip';
  import { History, Maximize2, MessageSquare, Minimize2, PanelRight, Sparkles, X } from 'lucide-vue-next';
  import { computed } from 'vue';
  import { useSvedaT } from '../../i18n/index';

  const t = useSvedaT();

  const props = defineProps({
    isMobile: { type: Boolean, required: true },
    viewMode: { type: String, required: true },
    isImmersiveDesktop: { type: Boolean, required: true },
    showHistorySidebar: { type: Boolean, required: true },
    immersiveUiEnabled: { type: Boolean, default: true },
    viewModeToggleEnabled: { type: Boolean, default: true },
    title: { type: String, required: true },
    viewModeFloatingLabel: { type: String, required: true },
    viewModeFixedLabel: { type: String, required: true },
    immersiveEnterLabel: { type: String, required: true },
    immersiveExitLabel: { type: String, required: true },
    immersiveBrandName: { type: String, default: '' },
    immersiveBrandLogo: { type: String, default: null },
  });

  defineEmits([
    'toggle-history-sidebar',
    'exit-immersive',
    'enter-immersive',
    'toggle-view-mode',
    'new-chat',
    'minimize',
  ]);

  const headerClass = computed(() => {
    const base =
      'flex h-14 shrink-0 flex-row items-center justify-between border-b border-border/50 bg-background/70 px-4 backdrop-blur supports-[backdrop-filter]:bg-background/70 md:px-6 xl:h-16';
    if (props.viewMode === 'immersive') {
      return `${base} rounded-none`;
    }
    if (props.isMobile || props.viewMode === 'fixed') {
      return `${base} rounded-none`;
    }
    return `${base} rounded-t-[var(--sveda-radius)]`;
  });
</script>

<template>
  <TooltipProvider>
    <header :class="headerClass">
      <div
        v-if="!isImmersiveDesktop"
        class="flex min-w-0 flex-1 items-center gap-2"
      >
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              variant="ghostTransparent"
              size="icon"
              class="h-9 w-9 text-muted-foreground hover:text-foreground"
              :class="{ 'bg-muted/60 text-foreground': showHistorySidebar }"
              @click="$emit('toggle-history-sidebar')"
            >
              <History class="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{{ t('chatHistory') }}</p>
          </TooltipContent>
        </Tooltip>
        <h2 class="min-w-0 flex-1 truncate text-left text-sm font-medium leading-tight text-foreground xl:text-base">
          {{ title }}
        </h2>
      </div>
      <div
        v-else
        class="flex min-w-0 flex-1 items-center gap-4"
      >
        <div class="w-[min(20rem,33vw)] min-w-0">
          <div class="flex items-center gap-2">
            <img
              v-if="immersiveBrandLogo"
              :src="immersiveBrandLogo"
              :alt="immersiveBrandName"
              class="size-7 shrink-0 rounded"
            />
            <Sparkles
              v-else
              class="size-5 shrink-0 text-primary"
            />
            <span class="truncate text-lg font-bold leading-none text-foreground">
              {{ immersiveBrandName }}
            </span>
          </div>
        </div>
        <h2 class="min-w-0 flex-1 truncate text-left text-sm font-medium leading-tight text-foreground xl:text-base">
          {{ title }}
        </h2>
      </div>
      <div class="flex shrink-0 items-center gap-0.5">
        <Tooltip v-if="immersiveUiEnabled && !isMobile && viewMode === 'immersive'">
          <TooltipTrigger as-child>
            <Button
              variant="ghostTransparent"
              size="icon"
              class="h-9 w-9 text-muted-foreground hover:text-foreground"
              @click="$emit('exit-immersive')"
            >
              <Minimize2 class="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{{ immersiveExitLabel }}</p>
          </TooltipContent>
        </Tooltip>
        <Tooltip v-if="immersiveUiEnabled && !isMobile && viewMode !== 'immersive'">
          <TooltipTrigger as-child>
            <Button
              variant="ghostTransparent"
              size="icon"
              class="h-9 w-9 text-muted-foreground hover:text-foreground"
              @click="$emit('enter-immersive')"
            >
              <Maximize2 class="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{{ immersiveEnterLabel }}</p>
          </TooltipContent>
        </Tooltip>
        <Tooltip v-if="viewModeToggleEnabled && !isMobile && viewMode !== 'immersive'">
          <TooltipTrigger as-child>
            <Button
              variant="ghostTransparent"
              size="icon"
              class="h-9 w-9 text-muted-foreground hover:text-foreground"
              @click="$emit('toggle-view-mode')"
            >
              <PanelRight
                v-if="viewMode === 'floating'"
                class="h-4 w-4"
              />
              <MessageSquare
                v-else
                class="h-4 w-4"
              />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{{ viewMode === 'floating' ? viewModeFixedLabel : viewModeFloatingLabel }}</p>
          </TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              variant="ghostTransparent"
              size="icon"
              class="h-9 w-9 text-muted-foreground hover:text-foreground"
              @click="$emit('minimize')"
            >
              <X class="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{{ t('toolbarMinimize') }}</p>
          </TooltipContent>
        </Tooltip>
      </div>
    </header>
  </TooltipProvider>
</template>
