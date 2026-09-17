import { inject, onUnmounted, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { SvedaClientKey } from '../plugin';

export function useSvedaReadable(
  description: MaybeRefOrGetter<string>,
  value: MaybeRefOrGetter<unknown>
): void {
  const client = inject(SvedaClientKey, null);
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
