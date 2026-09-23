import { SvedaClient } from '@sveda-ai/core';
import {
  applySvedaAppearance,
  mergeSvedaAppearance,
  type SvedaAppearance,
} from './appearance.js';
import { createSvedaI18n, type SvedaI18n, type SvedaMessages } from './i18n.js';
import { chatComponentI18nKeys, enMessages as en, ruMessages as ru } from '@sveda-ai/chat';

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

export type SvedaBeforeSend = () => void | Promise<void>;

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
  beforeSend?: SvedaBeforeSend;
}

export interface SvedaConfig {
  brand: {
    name: string;
    logoUrl: string | null;
  };
  models: SvedaModelOption[];
  quickPrompts: SvedaQuickPrompt[];
}

export interface SvedaContext {
  client: SvedaClient;
  i18n: SvedaI18n;
  config: SvedaConfig;
  hostEmbed: boolean;
  fillHost: boolean;
  hideLauncher: boolean;
  beforeSend: SvedaBeforeSend | null;
}

export const SVEDA_CONTEXT_KEY = 'sveda';

export const SVEDA_EMBED_AUTH_EVENT = 'sveda-embed-auth';

const DEFAULT_CONFIG: SvedaConfig = {
  brand: { name: 'Sveda', logoUrl: null },
  models: [],
  quickPrompts: [],
};

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

export function createSveda(options: SvedaPluginOptions): SvedaContext {
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

export function getDefaultSvedaConfig(): SvedaConfig {
  return DEFAULT_CONFIG;
}
