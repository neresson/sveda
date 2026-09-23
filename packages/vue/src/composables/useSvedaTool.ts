import type { SvedaFrontendTool } from '@sveda-ai/core';
import { inject, onUnmounted } from 'vue';
import { SvedaClientKey } from '../plugin';

export type UseSvedaToolOptions = SvedaFrontendTool;

export function useSvedaTool(tool: UseSvedaToolOptions): void {
  const client = inject(SvedaClientKey, null);
  if (!client) {
    return;
  }

  const unregister = client.toolRegistry.register(tool);

  onUnmounted(() => {
    unregister();
  });
}
