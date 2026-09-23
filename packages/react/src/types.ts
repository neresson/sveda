import type { SvedaClient } from '@sveda-ai/core';
import type { SvedaI18n } from './i18n';

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

export interface SvedaProviderOptions {
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
  appearance?: import('./appearance').SvedaAppearance | null;
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

export interface SvedaContextValue {
  client: SvedaClient;
  i18n: SvedaI18n;
  config: SvedaConfig;
  hostEmbed: boolean;
  fillHost: boolean;
  hideLauncher: boolean;
  beforeSend: SvedaBeforeSend | null;
}

export const SVEDA_EMBED_AUTH_EVENT = 'sveda-embed-auth';

export const DEFAULT_CONFIG: SvedaConfig = {
  brand: { name: 'Sveda', logoUrl: null },
  models: [],
  quickPrompts: [],
};
