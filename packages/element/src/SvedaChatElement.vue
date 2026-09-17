<script setup lang="ts">
import { applySvedaAppearance, Toaster, useSvedaChat, SvedaChat, SvedaFillHostKey } from '@sveda-ai/vue';
import { computed, onMounted, provide, ref, watch } from 'vue';
import type { SvedaSessionPayload } from './session';

const props = defineProps<{
  session: SvedaSessionPayload;
  brandName?: string;
  startOpen?: boolean;
}>();

provide(SvedaFillHostKey, true);

const rootRef = ref<HTMLElement | null>(null);
const brand = computed(() => props.brandName || 'Sveda');
const pageUrl = computed(() => (typeof window === 'undefined' ? '' : window.location.href));
const { maximizeChat, minimizeChat, isMinimized } = useSvedaChat();

const open = () => {
  maximizeChat();
};

const close = () => {
  minimizeChat();
};

const toggle = () => {
  if (isMinimized.value) {
    maximizeChat();
    return;
  }

  minimizeChat();
};

const syncHostOpenState = () => {
  const host = rootRef.value?.parentElement;
  if (!(host instanceof HTMLElement)) {
    return;
  }

  host.setAttribute('data-sveda-open', isMinimized.value ? 'false' : 'true');
};

onMounted(() => {
  if (props.session.appearance) {
    applySvedaAppearance(props.session.appearance);
  }

  if (props.startOpen !== false) {
    maximizeChat();
  }

  syncHostOpenState();
});

watch(isMinimized, syncHostOpenState);

defineExpose({ open, close, toggle });
</script>

<template>
  <div
    ref="rootRef"
    class="sveda-chat relative flex h-full min-h-0 w-full flex-col"
  >
    <SvedaChat :brand-name="brand" :page-url="pageUrl" />
    <Toaster v-if="!isMinimized" />
  </div>
</template>
