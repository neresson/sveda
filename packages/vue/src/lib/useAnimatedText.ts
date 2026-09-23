import { onUnmounted, ref, toValue, watch, type MaybeRefOrGetter, type Ref } from 'vue';

const CHAR_INTERVAL_MS = 12;
const CHARS_PER_TICK = 2;

export interface UseAnimatedTextOptions {
  intervalMs?: number;
  charsPerTick?: number;
  enabled?: MaybeRefOrGetter<boolean>;
}

export function useAnimatedText(
  getSource: MaybeRefOrGetter<string>,
  options: UseAnimatedTextOptions = {}
): Ref<string> {
  const intervalMs = options.intervalMs ?? CHAR_INTERVAL_MS;
  const charsPerTick = options.charsPerTick ?? CHARS_PER_TICK;
  const getEnabled = options.enabled ?? (() => true);
  const displayedText = ref('');
  let tickId: ReturnType<typeof setInterval> | null = null;

  function animateTowardsTarget() {
    const target = toValue(getSource);
    const targetStr = typeof target === 'string' ? target : '';
    if (displayedText.value.length >= targetStr.length) {
      if (tickId) {
        clearInterval(tickId);
        tickId = null;
      }
      return;
    }
    const remaining = targetStr.length - displayedText.value.length;
    const toAdd = Math.min(charsPerTick, remaining);
    displayedText.value = targetStr.slice(0, displayedText.value.length + toAdd);
  }

  watch(
    () => {
      const source = toValue(getSource);
      const enabled = toValue(getEnabled);
      return { source, enabled };
    },
    ({ source, enabled }) => {
      const newText = typeof source === 'string' ? source : '';
      if (!enabled) {
        displayedText.value = newText;
        if (tickId) {
          clearInterval(tickId);
          tickId = null;
        }
        return;
      }
      if (newText.length <= displayedText.value.length) {
        displayedText.value = newText;
        if (tickId) {
          clearInterval(tickId);
          tickId = null;
        }
        return;
      }
      if (!tickId) {
        tickId = setInterval(() => {
          animateTowardsTarget();
        }, intervalMs);
      }
    },
    { immediate: true }
  );

  onUnmounted(() => {
    if (tickId) {
      clearInterval(tickId);
      tickId = null;
    }
  });

  return displayedText;
}
