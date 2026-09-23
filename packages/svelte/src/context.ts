import { getContext, setContext } from 'svelte';
import {
  createSveda,
  getDefaultSvedaConfig,
  SVEDA_CONTEXT_KEY,
  type SvedaConfig,
  type SvedaContext,
  type SvedaPluginOptions,
} from './provider.js';
import type { SvedaClient } from '@sveda-ai/core';

export function setSvedaContext(ctx: SvedaContext): SvedaContext {
  setContext(SVEDA_CONTEXT_KEY, ctx);
  return ctx;
}

export function createSvedaContext(options: SvedaPluginOptions): SvedaContext {
  return setSvedaContext(createSveda(options));
}

export function useSvedaContext(): SvedaContext {
  const ctx = getContext<SvedaContext | undefined>(SVEDA_CONTEXT_KEY);
  if (!ctx) {
    throw new Error(
      '[sveda] Sveda context is missing. Wrap your tree in <SvedaProvider> or call setSvedaContext(createSveda(...)).',
    );
  }
  return ctx;
}

export function useSvedaClient(): SvedaClient {
  return useSvedaContext().client;
}

export function useSvedaConfig(): SvedaConfig {
  try {
    return useSvedaContext().config;
  } catch {
    return getDefaultSvedaConfig();
  }
}

export function useSvedaT(): (key: string, params?: Record<string, unknown>) => string {
  try {
    const { i18n } = useSvedaContext();
    return (key, params) => i18n.t(key, params);
  } catch {
    return (key) => key;
  }
}
