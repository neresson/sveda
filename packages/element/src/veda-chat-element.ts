import type { VedaChatSession, VedaClient } from '@veda-ai/core';
import vedaBaseStyles from '@veda-ai/vue/styles.css?inline';
import {
  createVeda,
  useVedaChat,
  useVedaClient,
  useVedaStreaming,
  VedaChat,
  type VedaModelOption,
  type VedaPluginOptions,
  type VedaQuickPrompt,
} from '@veda-ai/vue';
import {
  createApp,
  defineComponent,
  h,
  nextTick,
  onBeforeUnmount,
  reactive,
  watch,
  type App,
} from 'vue';

export type VedaChatElementConfig = Omit<Partial<VedaPluginOptions>, 'endpoints'> & {
  endpoints?: Partial<VedaPluginOptions['endpoints']>;
  styles?: string | false;
};

export type VedaChatElementTheme = 'light' | 'dark';

export interface VedaChatElementRootApi {
  open(): void;
  close(): void;
  sendMessage(text: string, context?: Record<string, unknown>): Promise<void>;
  getSession(): VedaChatSession | null;
  getClient(): VedaClient;
}

interface VedaElementState {
  brandName?: string;
  brandLogo?: string;
  pageUrl?: string;
  models?: VedaModelOption[];
  quickPrompts?: VedaQuickPrompt[];
}

type VedaElementEmit = (type: string, detail?: unknown) => void;

const serializeError = (payload: unknown): { message: string } => {
  if (payload instanceof Error) {
    return { message: payload.message };
  }
  if (typeof payload === 'object' && payload !== null && 'message' in payload) {
    return { message: String((payload as { message: unknown }).message) };
  }
  return { message: String(payload) };
};

function createVedaElementRoot(state: VedaElementState, emit: VedaElementEmit) {
  return defineComponent({
    name: 'VedaElementRoot',
    setup(_props, { expose }) {
      const store = useVedaChat();
      const client = useVedaClient();
      const streaming = useVedaStreaming(store, () => {}, {});

      const sessionSubscriptions = new Map<string, Array<() => void>>();

      const subscribeSession = (chatId: string): void => {
        if (sessionSubscriptions.has(chatId)) {
          return;
        }
        const session = client.session(chatId);
        sessionSubscriptions.set(chatId, [
          session.on('messages', () =>
            emit('veda-message', { chatId, messages: session.messages })
          ),
          session.on('status', payload => emit('veda-status', { chatId, status: payload })),
          session.on('error', payload =>
            emit('veda-error', { chatId, error: serializeError(payload) })
          ),
          session.on('toolProgress', payload =>
            emit('veda-tool-progress', { chatId, event: payload })
          ),
          session.on('finish', payload => emit('veda-finish', { chatId, event: payload })),
          session.on('title', payload => emit('veda-title', { chatId, title: payload })),
        ]);
      };

      watch(
        () => store.currentChat.value?.id ?? null,
        chatId => {
          if (chatId) {
            subscribeSession(chatId);
          }
        },
        { immediate: true }
      );

      onBeforeUnmount(() => {
        for (const unsubscribe of sessionSubscriptions.values()) {
          unsubscribe.forEach(off => off());
        }
        sessionSubscriptions.clear();
      });

      const sendMessage = async (
        text: string,
        context: Record<string, unknown> = {}
      ): Promise<void> => {
        if (!store.currentChat.value) {
          store.createNewChat();
          await nextTick();
        }
        const chat = store.currentChat.value;
        if (!chat) {
          return;
        }
        subscribeSession(chat.id);
        store.maximizeChat();
        await streaming.sendMessage(text, context);
      };

      expose({
        open: () => {
          void store.openChat();
        },
        close: () => store.closeChat(),
        sendMessage,
        getSession: () => {
          const chatId = store.currentChat.value?.id;
          return chatId ? client.session(chatId) : null;
        },
        getClient: () => client,
      } satisfies VedaChatElementRootApi);

      return () =>
        h(VedaChat, {
          brandName: state.brandName,
          brandLogo: state.brandLogo,
          pageUrl: state.pageUrl,
          models: state.models,
          quickPrompts: state.quickPrompts,
          onNavigate: (url: string) => emit('veda-navigate', { url }),
          notify: (kind: string, message: string) => emit('veda-notify', { kind, message }),
        });
    },
  });
}

const REMOUNT_ATTRIBUTES = new Set([
  'stream-endpoint',
  'message-endpoint',
  'histories-endpoint',
  'documents-extract-endpoint',
  'protocol-mode',
  'locale',
]);

export class VedaChatElement extends HTMLElement {
  static get observedAttributes(): string[] {
    return [
      'stream-endpoint',
      'message-endpoint',
      'histories-endpoint',
      'documents-extract-endpoint',
      'protocol-mode',
      'locale',
      'brand-name',
      'brand-logo-url',
      'page-url',
      'theme',
      'open',
      'minimized',
    ];
  }

  private app: App | null = null;

  private rootApi: VedaChatElementRootApi | null = null;

  private mountPoint: HTMLDivElement | null = null;

  private styleElement: HTMLStyleElement | null = null;

  private adoptedSheet: CSSStyleSheet | null = null;

  private pendingCss: string | null = null;

  private elementConfig: VedaChatElementConfig = {};

  private connected = false;

  private readonly state = reactive<VedaElementState>({});

  private readonly emit: VedaElementEmit = (type, detail) => {
    this.dispatchEvent(new CustomEvent(type, { detail, bubbles: true, composed: true }));
  };

  get config(): VedaChatElementConfig {
    return this.elementConfig;
  }

  set config(value: VedaChatElementConfig | null | undefined) {
    this.elementConfig = value ?? {};
    this.syncState();
    if (this.connected) {
      this.remount();
    }
  }

  connectedCallback(): void {
    if (!this.shadowRoot) {
      this.attachShadow({ mode: 'open' });
    }
    this.ensureMountPoint();
    this.syncState();
    this.applyTheme();
    if (this.pendingCss !== null) {
      this.setStyles(this.pendingCss);
    } else {
      const baseStyles = this.resolveBaseStyles();
      if (baseStyles !== null) {
        this.setStyles(baseStyles);
      }
    }
    this.connected = true;
    this.remount();
  }

  disconnectedCallback(): void {
    this.connected = false;
    this.unmount();
  }

  attributeChangedCallback(name: string, oldValue: string | null, newValue: string | null): void {
    if (oldValue === newValue) {
      return;
    }
    switch (name) {
      case 'brand-name':
      case 'brand-logo-url':
      case 'page-url':
        this.syncState();
        break;
      case 'theme':
        this.applyTheme();
        break;
      case 'open':
        if (newValue !== null) {
          this.rootApi?.open();
        }
        break;
      case 'minimized':
        if (newValue !== null) {
          this.rootApi?.close();
        }
        break;
      default:
        if (REMOUNT_ATTRIBUTES.has(name) && this.connected) {
          this.remount();
        }
    }
  }

  open(): void {
    this.rootApi?.open();
  }

  close(): void {
    this.rootApi?.close();
  }

  async sendMessage(text: string, context?: Record<string, unknown>): Promise<void> {
    if (!this.rootApi) {
      throw new Error('[veda] <veda-chat> is not mounted. Configure a stream endpoint first.');
    }
    await this.rootApi.sendMessage(text, context);
  }

  getSession(): VedaChatSession | null {
    return this.rootApi?.getSession() ?? null;
  }

  getClient(): VedaClient | null {
    return this.rootApi?.getClient() ?? null;
  }

  setStyles(css: string): void {
    this.pendingCss = css;
    const root = this.shadowRoot;
    if (!root) {
      return;
    }
    if (typeof CSSStyleSheet !== 'undefined' && 'adoptedStyleSheets' in root) {
      try {
        if (!this.adoptedSheet) {
          this.adoptedSheet = new CSSStyleSheet();
          root.adoptedStyleSheets = [...root.adoptedStyleSheets, this.adoptedSheet];
        }
        this.adoptedSheet.replaceSync(css);
        return;
      } catch {
        this.adoptedSheet = null;
      }
    }
    if (!this.styleElement) {
      this.styleElement = document.createElement('style');
      this.styleElement.setAttribute('data-veda-styles', '');
      root.appendChild(this.styleElement);
    }
    this.styleElement.textContent = css;
  }

  private resolveBaseStyles(): string | null {
    const configured = this.elementConfig.styles;
    if (configured === false) {
      return null;
    }
    if (typeof configured === 'string') {
      return configured;
    }
    return vedaBaseStyles;
  }

  private ensureMountPoint(): void {
    if (!this.shadowRoot || this.mountPoint) {
      return;
    }
    this.mountPoint = document.createElement('div');
    this.mountPoint.setAttribute('data-veda-root', '');
    this.shadowRoot.appendChild(this.mountPoint);
  }

  private syncState(): void {
    const config = this.elementConfig;
    this.state.brandName = this.getAttribute('brand-name') ?? config.brand?.name;
    this.state.brandLogo = this.getAttribute('brand-logo-url') ?? config.brand?.logoUrl;
    this.state.pageUrl = this.getAttribute('page-url') ?? undefined;
    this.state.models = config.models;
    this.state.quickPrompts = config.quickPrompts;
  }

  private applyTheme(): void {
    if (!this.mountPoint) {
      return;
    }
    const theme = this.getAttribute('theme');
    this.mountPoint.classList.toggle('dark', theme === 'dark');
    this.mountPoint.style.colorScheme =
      theme === 'dark' ? 'dark' : theme === 'light' ? 'light' : '';
  }

  private resolveOptions(): VedaPluginOptions {
    const config = this.elementConfig;
    const endpoint = (attribute: string, value?: string): string | undefined => {
      const fromAttribute = this.getAttribute(attribute);
      return fromAttribute ?? (value || undefined);
    };
    const endpoints = Object.fromEntries(
      Object.entries({
        stream: endpoint('stream-endpoint', config.endpoints?.stream),
        message: endpoint('message-endpoint', config.endpoints?.message),
        histories: endpoint('histories-endpoint', config.endpoints?.histories),
        documentsExtract: endpoint(
          'documents-extract-endpoint',
          config.endpoints?.documentsExtract
        ),
      }).filter(([, value]) => typeof value === 'string')
    ) as VedaPluginOptions['endpoints'];

    const protocolModeAttribute = this.getAttribute('protocol-mode');
    const protocolMode =
      protocolModeAttribute === 'vercel' || protocolModeAttribute === 'veda'
        ? protocolModeAttribute
        : config.protocolMode;

    return {
      ...config,
      endpoints,
      ...(protocolMode ? { protocolMode } : {}),
      locale: this.getAttribute('locale') ?? config.locale,
      brand: {
        name: this.getAttribute('brand-name') ?? config.brand?.name,
        logoUrl: this.getAttribute('brand-logo-url') ?? config.brand?.logoUrl,
      },
    };
  }

  private mount(): void {
    if (!this.mountPoint) {
      return;
    }
    const options = this.resolveOptions();
    if (!options.endpoints.stream) {
      return;
    }
    const veda = createVeda(options);
    const app = createApp(createVedaElementRoot(this.state, this.emit));
    app.use(veda);
    this.rootApi = app.mount(this.mountPoint) as unknown as VedaChatElementRootApi;
    this.app = app;

    if (this.hasAttribute('open')) {
      this.rootApi.open();
    } else if (this.hasAttribute('minimized')) {
      this.rootApi.close();
    }

    this.emit('veda-ready');
  }

  private unmount(): void {
    if (!this.app) {
      return;
    }
    this.app.unmount();
    this.app = null;
    this.rootApi = null;
  }

  private remount(): void {
    this.unmount();
    this.mount();
  }
}

export function defineVedaChatElement(tag = 'veda-chat'): void {
  if (typeof customElements === 'undefined') {
    return;
  }
  if (!customElements.get(tag)) {
    customElements.define(tag, VedaChatElement);
  }
}
