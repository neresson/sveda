export const SVEDA_EMBED_SOURCE = 'sveda-embed' as const;

export const SVEDA_EMBED_VERSION = 1 as const;

export interface SvedaEmbedEnvelope<T = unknown> {
  source: typeof SVEDA_EMBED_SOURCE;
  version: typeof SVEDA_EMBED_VERSION;
  type: string;
  payload: T;
}

export type SvedaEmbedHostCommandType =
  | 'ack'
  | 'setContext'
  | 'setAuthToken'
  | 'open'
  | 'close'
  | 'sendMessage'
  | 'setLocale'
  | 'setTheme';

export type SvedaEmbedEventType = 'ready' | 'resize' | 'navigate' | 'error' | 'toolProgress';

export type SvedaEmbedTheme = 'light' | 'dark';

export interface SvedaEmbedAckPayload {
  version: number;
}

export interface SvedaEmbedSetContextPayload {
  context: Record<string, unknown>;
}

export interface SvedaEmbedSetAuthTokenPayload {
  token: string | null;
}

export interface SvedaEmbedSendMessagePayload {
  text: string;
  context?: Record<string, unknown>;
}

export interface SvedaEmbedSetLocalePayload {
  locale: string;
}

export interface SvedaEmbedSetThemePayload {
  theme: SvedaEmbedTheme;
}

export interface SvedaEmbedReadyPayload {
  version: number;
}

export interface SvedaEmbedResizePayload {
  height: number;
}

export interface SvedaEmbedNavigatePayload {
  url: string;
}

export interface SvedaEmbedErrorPayload {
  message: string;
}

export interface SvedaEmbedToolProgressPayload {
  event: unknown;
}

const HOST_COMMAND_TYPES: readonly SvedaEmbedHostCommandType[] = [
  'ack',
  'setContext',
  'setAuthToken',
  'open',
  'close',
  'sendMessage',
  'setLocale',
  'setTheme',
];

const EMBED_EVENT_TYPES: readonly SvedaEmbedEventType[] = [
  'ready',
  'resize',
  'navigate',
  'error',
  'toolProgress',
];

export interface SvedaEmbedProtocol {
  encode<T>(type: string, payload: T): SvedaEmbedEnvelope<T>;
  decode(data: unknown): SvedaEmbedEnvelope | null;
  isSvedaEmbedMessage(data: unknown): data is SvedaEmbedEnvelope;
  isHostCommand(envelope: SvedaEmbedEnvelope): envelope is SvedaEmbedEnvelope & {
    type: SvedaEmbedHostCommandType;
  };
  isEmbedEvent(envelope: SvedaEmbedEnvelope): envelope is SvedaEmbedEnvelope & {
    type: SvedaEmbedEventType;
  };
}

export function createSvedaEmbedProtocol(): SvedaEmbedProtocol {
  const isSvedaEmbedMessage = (data: unknown): data is SvedaEmbedEnvelope => {
    if (typeof data !== 'object' || data === null) {
      return false;
    }
    const candidate = data as Partial<SvedaEmbedEnvelope>;
    return (
      candidate.source === SVEDA_EMBED_SOURCE &&
      candidate.version === SVEDA_EMBED_VERSION &&
      typeof candidate.type === 'string'
    );
  };

  return {
    encode<T>(type: string, payload: T): SvedaEmbedEnvelope<T> {
      return { source: SVEDA_EMBED_SOURCE, version: SVEDA_EMBED_VERSION, type, payload };
    },
    decode(data: unknown): SvedaEmbedEnvelope | null {
      return isSvedaEmbedMessage(data) ? data : null;
    },
    isSvedaEmbedMessage,
    isHostCommand(
      envelope: SvedaEmbedEnvelope
    ): envelope is SvedaEmbedEnvelope & { type: SvedaEmbedHostCommandType } {
      return HOST_COMMAND_TYPES.includes(envelope.type as SvedaEmbedHostCommandType);
    },
    isEmbedEvent(
      envelope: SvedaEmbedEnvelope
    ): envelope is SvedaEmbedEnvelope & { type: SvedaEmbedEventType } {
      return EMBED_EVENT_TYPES.includes(envelope.type as SvedaEmbedEventType);
    },
  };
}
