export const VEDA_EMBED_SOURCE = 'veda-embed' as const;

export const VEDA_EMBED_VERSION = 1 as const;

export interface VedaEmbedEnvelope<T = unknown> {
  source: typeof VEDA_EMBED_SOURCE;
  version: typeof VEDA_EMBED_VERSION;
  type: string;
  payload: T;
}

export type VedaEmbedHostCommandType =
  | 'ack'
  | 'setContext'
  | 'setAuthToken'
  | 'open'
  | 'close'
  | 'sendMessage'
  | 'setLocale'
  | 'setTheme';

export type VedaEmbedEventType = 'ready' | 'resize' | 'navigate' | 'error' | 'toolProgress';

export type VedaEmbedTheme = 'light' | 'dark';

export interface VedaEmbedAckPayload {
  version: number;
}

export interface VedaEmbedSetContextPayload {
  context: Record<string, unknown>;
}

export interface VedaEmbedSetAuthTokenPayload {
  token: string | null;
}

export interface VedaEmbedSendMessagePayload {
  text: string;
  context?: Record<string, unknown>;
}

export interface VedaEmbedSetLocalePayload {
  locale: string;
}

export interface VedaEmbedSetThemePayload {
  theme: VedaEmbedTheme;
}

export interface VedaEmbedReadyPayload {
  version: number;
}

export interface VedaEmbedResizePayload {
  height: number;
}

export interface VedaEmbedNavigatePayload {
  url: string;
}

export interface VedaEmbedErrorPayload {
  message: string;
}

export interface VedaEmbedToolProgressPayload {
  event: unknown;
}

const HOST_COMMAND_TYPES: readonly VedaEmbedHostCommandType[] = [
  'ack',
  'setContext',
  'setAuthToken',
  'open',
  'close',
  'sendMessage',
  'setLocale',
  'setTheme',
];

const EMBED_EVENT_TYPES: readonly VedaEmbedEventType[] = [
  'ready',
  'resize',
  'navigate',
  'error',
  'toolProgress',
];

export interface VedaEmbedProtocol {
  encode<T>(type: string, payload: T): VedaEmbedEnvelope<T>;
  decode(data: unknown): VedaEmbedEnvelope | null;
  isVedaEmbedMessage(data: unknown): data is VedaEmbedEnvelope;
  isHostCommand(envelope: VedaEmbedEnvelope): envelope is VedaEmbedEnvelope & {
    type: VedaEmbedHostCommandType;
  };
  isEmbedEvent(envelope: VedaEmbedEnvelope): envelope is VedaEmbedEnvelope & {
    type: VedaEmbedEventType;
  };
}

export function createVedaEmbedProtocol(): VedaEmbedProtocol {
  const isVedaEmbedMessage = (data: unknown): data is VedaEmbedEnvelope => {
    if (typeof data !== 'object' || data === null) {
      return false;
    }
    const candidate = data as Partial<VedaEmbedEnvelope>;
    return (
      candidate.source === VEDA_EMBED_SOURCE &&
      candidate.version === VEDA_EMBED_VERSION &&
      typeof candidate.type === 'string'
    );
  };

  return {
    encode<T>(type: string, payload: T): VedaEmbedEnvelope<T> {
      return { source: VEDA_EMBED_SOURCE, version: VEDA_EMBED_VERSION, type, payload };
    },
    decode(data: unknown): VedaEmbedEnvelope | null {
      return isVedaEmbedMessage(data) ? data : null;
    },
    isVedaEmbedMessage,
    isHostCommand(
      envelope: VedaEmbedEnvelope
    ): envelope is VedaEmbedEnvelope & { type: VedaEmbedHostCommandType } {
      return HOST_COMMAND_TYPES.includes(envelope.type as VedaEmbedHostCommandType);
    },
    isEmbedEvent(
      envelope: VedaEmbedEnvelope
    ): envelope is VedaEmbedEnvelope & { type: VedaEmbedEventType } {
      return EMBED_EVENT_TYPES.includes(envelope.type as VedaEmbedEventType);
    },
  };
}
