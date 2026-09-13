import type { VedaFrontendTool } from '@veda-ai/core';
import { inject, onUnmounted } from 'vue';
import { VedaClientKey } from '../plugin';

export type UseVedaToolOptions = VedaFrontendTool;

export function useVedaTool(tool: UseVedaToolOptions): void {
  const client = inject(VedaClientKey, null);
  if (!client) {
    return;
  }

  const unregister = client.toolRegistry.register(tool);

  onUnmounted(() => {
    unregister();
  });
}
