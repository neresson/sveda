<script setup>
  import { SvedaI18nKey, useSvedaT } from '../../i18n/index.js';
  import { buildPendingStatusRotation } from '../../lib/pendingStatusRotation.js';
  import { computed, inject, onUnmounted, ref, watch } from 'vue';

  const props = defineProps({
    active: {
      type: Boolean,
      default: false,
    },
    intervalMs: {
      type: Number,
      default: 3200,
    },
    statusMessages: {
      type: Array,
      default: () => [],
    },
  });

  const t = useSvedaT();
  const svedaI18n = inject(SvedaI18nKey, null);
  const locale = computed(() => svedaI18n?.locale ?? '');

  const sourceStatusMessages = computed(() => {
    const provided = props.statusMessages.filter(
      message => typeof message === 'string' && message.trim() !== ''
    );
    if (provided.length > 0) {
      return provided;
    }

    const fallback = t('pendingStatus');
    return fallback && fallback !== 'pendingStatus' ? [fallback] : [];
  });

  const rotationMessages = ref([]);
  const currentIndex = ref(0);
  let rotationTimer = null;

  const rebuildRotationMessages = () => {
    rotationMessages.value = buildPendingStatusRotation(sourceStatusMessages.value);
    currentIndex.value = 0;
  };

  const currentMessage = computed(() => {
    const messages = rotationMessages.value;
    if (messages.length === 0) {
      return '';
    }

    return messages[currentIndex.value % messages.length];
  });

  const stopRotation = () => {
    if (rotationTimer !== null) {
      clearInterval(rotationTimer);
      rotationTimer = null;
    }
  };

  const startRotation = () => {
    stopRotation();
    if (!props.active || rotationMessages.value.length <= 1) {
      return;
    }

    rotationTimer = setInterval(() => {
      currentIndex.value = (currentIndex.value + 1) % rotationMessages.value.length;
    }, props.intervalMs);
  };

  watch(
    () => props.active,
    active => {
      if (active) {
        rebuildRotationMessages();
        startRotation();
      } else {
        stopRotation();
        rotationMessages.value = [];
        currentIndex.value = 0;
      }
    },
    { immediate: true }
  );

  watch(sourceStatusMessages, () => {
    if (props.active) {
      rebuildRotationMessages();
      startRotation();
    }
  });

  watch(locale, () => {
    if (props.active) {
      rebuildRotationMessages();
      startRotation();
    }
  });

  onUnmounted(stopRotation);
</script>

<template>
  <div
    v-if="active && currentMessage"
    class="flex justify-start"
    data-testid="chat-pending-indicator"
  >
    <div class="max-w-[85%] rounded-[var(--sveda-radius)] border border-border bg-muted px-4 py-3">
      <div class="flex items-center gap-3">
        <div
          class="flex shrink-0 items-center space-x-1"
          aria-hidden="true"
        >
          <div
            class="h-2 w-2 animate-bounce rounded-full bg-foreground/40"
            style="animation-delay: 0ms"
          />
          <div
            class="h-2 w-2 animate-bounce rounded-full bg-foreground/40"
            style="animation-delay: 150ms"
          />
          <div
            class="h-2 w-2 animate-bounce rounded-full bg-foreground/40"
            style="animation-delay: 300ms"
          />
        </div>
        <Transition
          name="pending-status"
          mode="out-in"
        >
          <p
            :key="currentMessage"
            class="text-sm text-muted-foreground"
          >
            {{ currentMessage }}
          </p>
        </Transition>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .pending-status-enter-active,
  .pending-status-leave-active {
    transition:
      opacity 0.22s ease,
      transform 0.22s ease;
  }

  .pending-status-enter-from {
    opacity: 0;
    transform: translateY(4px);
  }

  .pending-status-leave-to {
    opacity: 0;
    transform: translateY(-4px);
  }
</style>
