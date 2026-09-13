import { VedaClient } from '@veda-ai/core';
import { inject, type App, type InjectionKey } from 'vue';
import {
  createVedaI18n,
  installVedaI18n,
  type VedaI18n,
  type VedaMessages,
} from './i18n/index';
import en from './i18n/locales/en';
import ru from './i18n/locales/ru';
import { chatComponentI18nKeys } from './lib/chatI18nKeys';

export interface VedaModelOption {
  id: string;
  label: string;
  supportsThinking?: boolean;
}

export interface VedaQuickPrompt {
  label: string;
  prompt: string;
}

export interface VedaBrand {
  name?: string;
  logoUrl?: string;
}

export interface VedaPluginOptions {
  endpoints: {
    stream: string;
    message?: string;
    histories?: string;
    documentsExtract?: string;
  };
  protocolMode?: 'veda' | 'vercel';
  headers?: Record<string, string> | (() => Record<string, string>);
  credentials?: RequestCredentials;
  locale?: string;
  messages?: Record<string, Record<string, string>>;
  brand?: VedaBrand;
  models?: VedaModelOption[];
  quickPrompts?: VedaQuickPrompt[];
}

export interface VedaConfig {
  brand: {
    name: string;
    logoUrl: string | null;
  };
  models: VedaModelOption[];
  quickPrompts: VedaQuickPrompt[];
}

export interface VedaPlugin {
  client: VedaClient;
  i18n: VedaI18n;
  config: VedaConfig;
  install(app: App): void;
}

export const VedaClientKey: InjectionKey<VedaClient> = Symbol('veda-client');

export const VedaConfigKey: InjectionKey<VedaConfig> = Symbol('veda-config');

const DEFAULT_CONFIG: VedaConfig = {
  brand: { name: 'Veda', logoUrl: null },
  models: [],
  quickPrompts: [],
};

export function createVeda(options: VedaPluginOptions): VedaPlugin {
  const client = new VedaClient({
    endpoints: options.endpoints,
    protocolMode: options.protocolMode,
    headers: options.headers,
    credentials: options.credentials,
  });

  const messages: Record<string, VedaMessages> = {};
  const mergeMessages = (locale: string, localeMessages: VedaMessages) => {
    messages[locale] = { ...(messages[locale] ?? {}), ...localeMessages };
  };

  for (const [locale, localeMessages] of Object.entries({ en, ru })) {
    mergeMessages(locale, localeMessages);
  }
  for (const [locale, localeMessages] of Object.entries(chatComponentI18nKeys)) {
    mergeMessages(locale, localeMessages);
  }
  for (const [locale, localeMessages] of Object.entries(options.messages ?? {})) {
    mergeMessages(locale, localeMessages);
  }

  const i18n = createVedaI18n({
    locale: options.locale ?? 'en',
    messages,
  });

  const config: VedaConfig = {
    brand: {
      name: options.brand?.name ?? 'Veda',
      logoUrl: options.brand?.logoUrl ?? null,
    },
    models: options.models ?? [],
    quickPrompts: options.quickPrompts ?? [],
  };

  return {
    client,
    i18n,
    config,
    install(app: App) {
      app.provide(VedaClientKey, client);
      app.provide(VedaConfigKey, config);
      installVedaI18n(app, i18n);
    },
  };
}

export function useVedaClient(): VedaClient {
  const client = inject(VedaClientKey, null);
  if (!client) {
    throw new Error('[veda] Veda plugin is not installed. Call app.use(createVeda(...)) first.');
  }

  return client;
}

export function useVedaConfig(): VedaConfig {
  return inject(VedaConfigKey, DEFAULT_CONFIG);
}

const vedaPlugin = {
  install(app: App, options: VedaPluginOptions) {
    createVeda(options).install(app);
  },
};

export default vedaPlugin;
