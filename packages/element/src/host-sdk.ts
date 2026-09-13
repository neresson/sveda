import {
  createVedaEmbedProtocol,
  VEDA_EMBED_VERSION,
  type VedaEmbedEnvelope,
  type VedaEmbedErrorPayload,
  type VedaEmbedNavigatePayload,
  type VedaEmbedResizePayload,
  type VedaEmbedTheme,
  type VedaEmbedToolProgressPayload,
} from './embed-protocol';

export interface ConnectVedaEmbedOptions {
  allowedOrigins: string[];
  onReady?: () => void;
  onResize?: (height: number) => void;
  onNavigate?: (url: string) => void;
  onError?: (error: VedaEmbedErrorPayload) => void;
  onToolProgress?: (event: unknown) => void;
}

export interface VedaEmbedConnection {
  readonly ready: boolean;
  setContext(context: Record<string, unknown>): void;
  setAuthToken(token: string | null): void;
  open(): void;
  close(): void;
  sendMessage(text: string, context?: Record<string, unknown>): void;
  setLocale(locale: string): void;
  setTheme(theme: VedaEmbedTheme): void;
  disconnect(): void;
}

const resolveIframeOrigin = (iframe: HTMLIFrameElement): string | null => {
  const src = iframe.src;
  if (!src) {
    return null;
  }
  try {
    return new URL(src, window.location.href).origin;
  } catch {
    return null;
  }
};

export function connectVedaEmbed(
  iframe: HTMLIFrameElement,
  options: ConnectVedaEmbedOptions
): VedaEmbedConnection {
  const protocol = createVedaEmbedProtocol();
  const allowedOrigins = new Set(options.allowedOrigins);
  const iframeOrigin = resolveIframeOrigin(iframe);

  let ready = false;
  let embedOrigin: string | null = null;
  const queue: Array<VedaEmbedEnvelope> = [];

  const post = (envelope: VedaEmbedEnvelope): void => {
    const target = iframe.contentWindow;
    if (!target) {
      return;
    }
    target.postMessage(envelope, embedOrigin ?? iframeOrigin ?? '*');
  };

  const send = (type: string, payload: unknown): void => {
    const envelope = protocol.encode(type, payload);
    if (!ready) {
      queue.push(envelope);
      return;
    }
    post(envelope);
  };

  const flush = (): void => {
    while (queue.length > 0) {
      post(queue.shift()!);
    }
  };

  const handleMessage = (event: MessageEvent): void => {
    if (event.source !== iframe.contentWindow) {
      return;
    }
    if (!allowedOrigins.has(event.origin)) {
      return;
    }
    const envelope = protocol.decode(event.data);
    if (!envelope || !protocol.isEmbedEvent(envelope)) {
      return;
    }

    switch (envelope.type) {
      case 'ready':
        ready = true;
        embedOrigin = event.origin;
        post(protocol.encode('ack', { version: VEDA_EMBED_VERSION }));
        flush();
        options.onReady?.();
        break;
      case 'resize': {
        const payload = envelope.payload as VedaEmbedResizePayload;
        if (typeof payload?.height === 'number') {
          options.onResize?.(payload.height);
        }
        break;
      }
      case 'navigate': {
        const payload = envelope.payload as VedaEmbedNavigatePayload;
        if (typeof payload?.url === 'string' && payload.url) {
          options.onNavigate?.(payload.url);
        }
        break;
      }
      case 'error':
        options.onError?.(envelope.payload as VedaEmbedErrorPayload);
        break;
      case 'toolProgress':
        options.onToolProgress?.((envelope.payload as VedaEmbedToolProgressPayload)?.event);
        break;
    }
  };

  window.addEventListener('message', handleMessage);

  return {
    get ready() {
      return ready;
    },
    setContext(context) {
      send('setContext', { context });
    },
    setAuthToken(token) {
      send('setAuthToken', { token });
    },
    open() {
      send('open', {});
    },
    close() {
      send('close', {});
    },
    sendMessage(text, context) {
      send('sendMessage', { text, context });
    },
    setLocale(locale) {
      send('setLocale', { locale });
    },
    setTheme(theme) {
      send('setTheme', { theme });
    },
    disconnect() {
      window.removeEventListener('message', handleMessage);
      queue.length = 0;
      ready = false;
    },
  };
}
