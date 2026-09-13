import {
  createVedaEmbedProtocol,
  VEDA_EMBED_VERSION,
  type VedaEmbedSendMessagePayload,
  type VedaEmbedSetAuthTokenPayload,
  type VedaEmbedSetContextPayload,
  type VedaEmbedSetLocalePayload,
  type VedaEmbedSetThemePayload,
} from './embed-protocol';

export interface VedaEmbedHostHandlers {
  onAck?: () => void;
  onSetContext?: (context: Record<string, unknown>) => void;
  onSetAuthToken?: (token: string | null) => void;
  onOpen?: () => void;
  onClose?: () => void;
  onSendMessage?: (text: string, context?: Record<string, unknown>) => void;
  onSetLocale?: (locale: string) => void;
  onSetTheme?: (theme: string) => void;
}

export interface VedaEmbedHostOptions {
  allowedOrigins?: string[];
  target?: Window;
  targetOrigin?: string;
  readyRetryInterval?: number;
  maxReadyAttempts?: number;
}

export interface VedaEmbedHost {
  readonly acknowledged: boolean;
  postReady(): void;
  postResize(height: number): void;
  postNavigate(url: string): void;
  postError(message: string): void;
  postToolProgress(event: unknown): void;
  disconnect(): void;
}

const DEFAULT_READY_RETRY_INTERVAL = 250;
const DEFAULT_MAX_READY_ATTEMPTS = 40;

export function initVedaEmbedHost(
  handlers: VedaEmbedHostHandlers = {},
  options: VedaEmbedHostOptions = {}
): VedaEmbedHost {
  const protocol = createVedaEmbedProtocol();
  const target = options.target ?? (window.parent !== window ? window.parent : null);
  const targetOrigin = options.targetOrigin ?? '*';
  const allowedOrigins = options.allowedOrigins ? new Set(options.allowedOrigins) : null;
  const retryInterval = options.readyRetryInterval ?? DEFAULT_READY_RETRY_INTERVAL;
  const maxAttempts = options.maxReadyAttempts ?? DEFAULT_MAX_READY_ATTEMPTS;

  let acknowledged = false;
  let attempts = 0;
  let retryTimer: ReturnType<typeof setInterval> | null = null;

  const post = (type: string, payload: unknown): void => {
    if (!target) {
      return;
    }
    target.postMessage(protocol.encode(type, payload), targetOrigin);
  };

  const stopRetry = (): void => {
    if (retryTimer !== null) {
      clearInterval(retryTimer);
      retryTimer = null;
    }
  };

  const postReady = (): void => {
    post('ready', { version: VEDA_EMBED_VERSION });
  };

  const handleMessage = (event: MessageEvent): void => {
    if (target && event.source !== target) {
      return;
    }
    if (allowedOrigins && !allowedOrigins.has(event.origin)) {
      return;
    }
    const envelope = protocol.decode(event.data);
    if (!envelope || !protocol.isHostCommand(envelope)) {
      return;
    }

    switch (envelope.type) {
      case 'ack':
        acknowledged = true;
        stopRetry();
        handlers.onAck?.();
        break;
      case 'setContext':
        handlers.onSetContext?.((envelope.payload as VedaEmbedSetContextPayload)?.context ?? {});
        break;
      case 'setAuthToken':
        handlers.onSetAuthToken?.((envelope.payload as VedaEmbedSetAuthTokenPayload)?.token ?? null);
        break;
      case 'open':
        handlers.onOpen?.();
        break;
      case 'close':
        handlers.onClose?.();
        break;
      case 'sendMessage': {
        const payload = envelope.payload as VedaEmbedSendMessagePayload;
        if (typeof payload?.text === 'string' && payload.text) {
          handlers.onSendMessage?.(payload.text, payload.context);
        }
        break;
      }
      case 'setLocale': {
        const payload = envelope.payload as VedaEmbedSetLocalePayload;
        if (typeof payload?.locale === 'string' && payload.locale) {
          handlers.onSetLocale?.(payload.locale);
        }
        break;
      }
      case 'setTheme': {
        const payload = envelope.payload as VedaEmbedSetThemePayload;
        if (payload?.theme === 'light' || payload?.theme === 'dark') {
          handlers.onSetTheme?.(payload.theme);
        }
        break;
      }
    }
  };

  window.addEventListener('message', handleMessage);

  postReady();
  if (!acknowledged && retryInterval > 0 && target) {
    retryTimer = setInterval(() => {
      attempts += 1;
      if (acknowledged || attempts >= maxAttempts) {
        stopRetry();
        return;
      }
      postReady();
    }, retryInterval);
  }

  return {
    get acknowledged() {
      return acknowledged;
    },
    postReady,
    postResize(height: number) {
      post('resize', { height });
    },
    postNavigate(url: string) {
      post('navigate', { url });
    },
    postError(message: string) {
      post('error', { message });
    },
    postToolProgress(event: unknown) {
      post('toolProgress', { event });
    },
    disconnect() {
      stopRetry();
      window.removeEventListener('message', handleMessage);
    },
  };
}
