import { SvedaClient } from '@sveda-ai/core';
import type { SvedaClient as SvedaClientInstance } from '@sveda-ai/core';
import {
  applySvedaAppearance,
  mergeSvedaAppearance,
  type SvedaAppearance,
} from './appearance';
import { chatComponentI18nKeys, enMessages, ruMessages } from '@sveda-ai/chat';
import {
  createContext,
  createElement,
  useContext,
  useMemo,
  type ReactNode,
} from 'react';
import { createSvedaI18n, type SvedaMessages } from './i18n';
import {
  DEFAULT_CONFIG,
  type SvedaConfig,
  type SvedaContextValue,
  type SvedaProviderOptions,
} from './types';

const SvedaContext = createContext<SvedaContextValue | null>(null);

const scheduleEmbedAppearance = (stream: string): void => {
  if (typeof fetch === 'undefined') {
    return;
  }

  let origin = '';
  try {
    const url = new URL(stream);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') {
      return;
    }
    origin = url.origin;
  } catch {
    return;
  }

  void fetch(`${origin}/sveda/embed/config`)
    .then((response) => (response.ok ? response.json() : null))
    .then((config) => {
      if (!config || typeof config !== 'object') {
        return;
      }

      applySvedaAppearance(
        mergeSvedaAppearance((config as { appearance?: SvedaAppearance | null }).appearance, undefined),
      );
    })
    .catch(() => {});
};

export function createSvedaContextValue(options: SvedaProviderOptions): SvedaContextValue {
  const client = new SvedaClient({
    endpoints: options.endpoints,
    protocolMode: options.protocolMode,
    headers: options.headers,
    credentials: options.credentials,
  });

  const messages: Record<string, SvedaMessages> = {};
  const mergeMessages = (locale: string, localeMessages: SvedaMessages) => {
    messages[locale] = { ...(messages[locale] ?? {}), ...localeMessages };
  };

  for (const [locale, localeMessages] of Object.entries({ en: enMessages, ru: ruMessages })) {
    mergeMessages(locale, localeMessages);
  }
  for (const [locale, localeMessages] of Object.entries(chatComponentI18nKeys)) {
    mergeMessages(locale, localeMessages);
  }
  for (const [locale, localeMessages] of Object.entries(options.messages ?? {})) {
    mergeMessages(locale, localeMessages);
  }

  const i18n = createSvedaI18n({
    locale: options.locale ?? 'en',
    messages,
  });

  const config: SvedaConfig = {
    brand: {
      name: options.brand?.name ?? 'Sveda',
      logoUrl: options.brand?.logoUrl ?? null,
    },
    models: options.models ?? [],
    quickPrompts: options.quickPrompts ?? [],
  };

  if (options.appearance !== undefined || options.theme !== undefined) {
    if (options.appearance === null && options.theme === undefined) {
      applySvedaAppearance(null);
    } else {
      applySvedaAppearance({
        ...(options.appearance && typeof options.appearance === 'object' ? options.appearance : {}),
        ...(options.theme ? { theme: options.theme } : {}),
      });
    }
  } else {
    scheduleEmbedAppearance(options.endpoints.stream);
  }

  return {
    client,
    i18n,
    config,
    hostEmbed: Boolean(options.hostEmbed),
    fillHost: Boolean(options.fillHost ?? options.hostEmbed),
    hideLauncher: Boolean(options.hideLauncher),
    beforeSend: options.beforeSend ?? null,
  };
}

export interface SvedaProviderProps extends SvedaProviderOptions {
  children: ReactNode;
}

export function SvedaProvider({ children, ...options }: SvedaProviderProps) {
  const value = useMemo(() => createSvedaContextValue(options), [
    options.endpoints.stream,
    options.endpoints.message,
    options.endpoints.histories,
    options.endpoints.documentsExtract,
    options.protocolMode,
    options.credentials,
    options.locale,
    options.theme,
    options.hostEmbed,
    options.fillHost,
    options.hideLauncher,
    options.brand?.name,
    options.brand?.logoUrl,
    options.appearance,
    options.headers,
    options.messages,
    options.models,
    options.quickPrompts,
    options.beforeSend,
  ]);

  return createElement(SvedaContext.Provider, { value }, children);
}

export function useSvedaClient(): SvedaClientInstance {
  const ctx = useContext(SvedaContext);
  if (!ctx) {
    throw new Error('[sveda] SvedaProvider is missing. Wrap your tree in <SvedaProvider> first.');
  }

  return ctx.client;
}

export function useSvedaConfig(): SvedaConfig {
  const ctx = useContext(SvedaContext);
  return ctx?.config ?? DEFAULT_CONFIG;
}

export function useSvedaContext(): SvedaContextValue {
  const ctx = useContext(SvedaContext);
  if (!ctx) {
    throw new Error('[sveda] SvedaProvider is missing. Wrap your tree in <SvedaProvider> first.');
  }

  return ctx;
}

export function useSvedaT(): (key: string, params?: Record<string, unknown>) => string {
  const ctx = useContext(SvedaContext);
  if (!ctx) {
    return (key: string) => key;
  }

  return (key, params) => ctx.i18n.t(key, params);
}

export { SvedaContext };
