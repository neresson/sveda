import { inject, onUnmounted, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { VedaClientKey } from '../plugin';

export function useVedaReadable(
  description: MaybeRefOrGetter<string>,
  value: MaybeRefOrGetter<unknown>
): void {
  const client = inject(VedaClientKey, null);
  if (!client) {
    return;
  }

  let unregister: (() => void) | null = null;

  const register = () => {
    unregister?.();
    unregister = client.contextRegistry.register(toValue(description), () => toValue(value));
  };

  register();

  watch(
    () => toValue(description),
    () => {
      register();
    }
  );

  onUnmounted(() => {
    unregister?.();
    unregister = null;
  });
}
