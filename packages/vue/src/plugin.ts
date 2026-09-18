import { SvedaClient } from '@sveda-ai/core';
import { inject, type App, type InjectionKey } from 'vue';
import { applySvedaAppearance, type SvedaAppearance } from './appearance';
import {
  createSvedaI18n,
  installSvedaI18n,
  type SvedaI18n,
  type SvedaMessages,
} from './i18n/index';
import en from './i18n/locales/en';
import ru from './i18n/locales/ru';
import { chatComponentI18nKeys } from './lib/chatI18nKeys';

export interface SvedaModelOption {
  id: string;
  label: string;
  supportsThinking?: boolean;
}

export interface SvedaQuickPrompt {
  label: string;
  prompt: string;
}

export interface SvedaBrand {
  name?: string;
  logoUrl?: string;
}

export interface SvedaPluginOptions {
  endpoints: {
    stream: string;
    message?: string;
    histories?: string;
    documentsExtract?: string;
  };
  protocolMode?: 'sveda' | 'vercel';
  headers?: Record<string, string> | (() => Record<string, string>);
  credentials?: RequestCredentials;
  locale?: string;
  messages?: Record<string, Record<string, string>>;
  brand?: SvedaBrand;
  models?: SvedaModelOption[];
  quickPrompts?: SvedaQuickPrompt[];
  appearance?: SvedaAppearance | null;
  theme?: 'light' | 'dark';
  hostEmbed?: boolean;
  fillHost?: boolean;
  hideLauncher?: boolean;
}

export interface SvedaConfig {
  brand: {
    name: string;
    logoUrl: string | null;
  };
  models: SvedaModelOption[];
  quickPrompts: SvedaQuickPrompt[];
}

export interface SvedaPlugin {
  client: SvedaClient;
  i18n: SvedaI18n;
  config: SvedaConfig;
  install(app: App): void;
}

export const SvedaClientKey: InjectionKey<SvedaClient> = Symbol('sveda-client');

export const SvedaConfigKey: InjectionKey<SvedaConfig> = Symbol('sveda-config');

export const SvedaHostEmbedKey: InjectionKey<boolean> = Symbol('sveda-host-embed');

export const SvedaFillHostKey: InjectionKey<boolean> = Symbol('sveda-fill-host');

export const SvedaHideLauncherKey: InjectionKey<boolean> = Symbol('sveda-hide-launcher');

const DEFAULT_CONFIG: SvedaConfig = {
  brand: { name: 'Sveda', logoUrl: null },
  models: [],
  quickPrompts: [],
};

export function createSveda(options: SvedaPluginOptions): SvedaPlugin {
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

  for (const [locale, localeMessages] of Object.entries({ en, ru })) {
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
  }

  return {
    client,
    i18n,
    config,
    install(app: App) {
      app.provide(SvedaClientKey, client);
      app.provide(SvedaConfigKey, config);
      app.provide(SvedaHostEmbedKey, Boolean(options.hostEmbed));
      app.provide(SvedaFillHostKey, Boolean(options.fillHost ?? options.hostEmbed));
      app.provide(SvedaHideLauncherKey, Boolean(options.hideLauncher));
      installSvedaI18n(app, i18n);
    },
  };
}

export function useSvedaClient(): SvedaClient {
  const client = inject(SvedaClientKey, null);
  if (!client) {
    throw new Error('[sveda] Sveda plugin is not installed. Call app.use(createSveda(...)) first.');
  }

  return client;
}

export function useSvedaConfig(): SvedaConfig {
  return inject(SvedaConfigKey, DEFAULT_CONFIG);
}

const svedaPlugin = {
  install(app: App, options: SvedaPluginOptions) {
    createSveda(options).install(app);
  },
};

export default svedaPlugin;
