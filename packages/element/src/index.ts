import { parseSvedaCapabilities } from '@sveda-ai/protocol';
import {
  applySvedaAppearance,
  createSveda,
  mergeSvedaAppearance,
  SVEDA_EMBED_AUTH_EVENT,
  type SvedaAppearance,
  type SvedaModelOption,
} from '@sveda-ai/vue';
import { createApp } from 'vue';
import './host.css';
import SvedaChatElement from './SvedaChatElement.vue';
import { parseSessionAttributes, requestHostSession, resolveSvedaSession } from './session';

const ELEMENT_TAG = 'sveda-chat';

type SvedaChatApi = {
  open: () => void;
  close: () => void;
  toggle: () => void;
  setToken: (token: string) => void;
};

type SvedaChatHost = HTMLElement &
  SvedaChatApi & {
    ready?: Promise<void>;
    beforeSend?: () => void | Promise<void>;
  };

const chatEndpoints = (origin: string, prefix = 'sveda') => {
  const base = origin.replace(/\/$/, '');
  const path = prefix.replace(/^\/+|\/+$/g, '') || 'sveda';

  return {
    stream: `${base}/${path}/stream`,
    message: `${base}/${path}/message`,
    histories: `${base}/${path}/chat-histories`,
    documentsExtract: `${base}/${path}/documents/extract`,
    config: `${base}/${path}/embed/config`,
  };
};

const buildModels = (models: unknown): SvedaModelOption[] => {
  if (!Array.isArray(models)) {
    return [];
  }

  const parsed: SvedaModelOption[] = [];

  for (const model of models) {
    const record = model as Record<string, unknown>;
    const id = String(record.id ?? '').trim();
    if (!id) {
      continue;
    }

    parsed.push({
      id,
      label: String(record.label ?? id),
      supportsThinking: Boolean(record.supportsThinking ?? record.supports_thinking ?? record.thinking),
    });
  }

  return parsed;
};

const loadEmbedConfig = async (origin: string, token: string, prefix = 'sveda') => {
  try {
    const response = await fetch(chatEndpoints(origin, prefix).config, {
      headers: {
        Accept: 'application/json',
        'X-Sveda-Embed-Token': token,
      },
    });

    if (!response.ok) {
      return {};
    }

    return await response.json();
  } catch {
    return {};
  }
};

const mountElement = async (element: HTMLElement): Promise<SvedaChatApi | null> => {
  if (element.dataset.svedaMounted === 'true') {
    return element as HTMLElement & SvedaChatApi;
  }

  const session = await resolveSvedaSession(element);
  if (!session) {
    console.error('Sveda chat: provide session URL or origin+token attributes.');
    return null;
  }

  const sessionState = { origin: session.origin, token: session.token };
  const hideLauncher = element.hasAttribute('hide-launcher');
  const config = sessionState.token
    ? await loadEmbedConfig(sessionState.origin, sessionState.token)
    : {};
  const appearance =
    mergeSvedaAppearance(
      (config as { appearance?: SvedaAppearance | null }).appearance,
      (session.appearance ?? undefined) as SvedaAppearance | null | undefined,
    ) ?? {};
  applySvedaAppearance(appearance);

  const plugin = createSveda({
    endpoints: chatEndpoints(sessionState.origin),
    protocolMode: 'sveda',
    credentials: 'omit',
    hostEmbed: true,
    hideLauncher,
    headers: () => ({
      'X-Requested-With': 'XMLHttpRequest',
      ...(sessionState.token ? { 'X-Sveda-Embed-Token': sessionState.token } : {}),
    }),
    beforeSend: async () => {
      await (element as SvedaChatHost).beforeSend?.();
    },
    locale: document.documentElement.lang || 'ru',
    brand: { name: (appearance as { brand?: { name?: string } })?.brand?.name || 'Sveda' },
    models: buildModels((config as { models?: unknown }).models),
    appearance,
  });
  plugin.client.toolRegistry.setCapabilities(
    parseSvedaCapabilities((config as { capabilities?: unknown }).capabilities),
  );

  const app = createApp(SvedaChatElement, {
    session: { ...session, appearance },
    brandName: (appearance as { brand?: { name?: string } })?.brand?.name,
    startOpen: !hideLauncher,
  });
  app.use(plugin);
  const instance = app.mount(element) as unknown as SvedaChatApi;
  const api: SvedaChatApi = {
    open: () => {
      instance.open();
      element.setAttribute('data-sveda-open', 'true');
    },
    close: () => {
      instance.close();
      element.setAttribute('data-sveda-open', 'false');
    },
    toggle: () => instance.toggle(),
    setToken: (token: string) => {
      sessionState.token = token.trim();
      if (sessionState.token) {
        element.setAttribute('token', sessionState.token);
        element.dispatchEvent(new CustomEvent(SVEDA_EMBED_AUTH_EVENT, { bubbles: true }));
      } else {
        element.removeAttribute('token');
      }
    },
  };
  Object.assign(element, api);
  element.dataset.svedaMounted = 'true';

  return api;
};

class SvedaChatCustomElement extends HTMLElement {
  static get observedAttributes(): string[] {
    return ['origin', 'token', 'session'];
  }

  ready: Promise<void>;

  #resolveReady: () => void = () => {};

  #mounting = false;

  constructor() {
    super();
    this.ready = new Promise((resolve) => {
      this.#resolveReady = resolve;
    });
  }

  connectedCallback(): void {
    void this.#mount();
  }

  attributeChangedCallback(name: string, oldValue: string | null, newValue: string | null): void {
    if (oldValue === newValue || !this.isConnected) {
      return;
    }
    if (name === 'origin' || name === 'token' || name === 'session') {
      void this.#mount();
    }
  }

  async #mount(): Promise<void> {
    if (this.#mounting || this.dataset.svedaMounted === 'true') {
      return;
    }
    this.#mounting = true;
    try {
      await mountElement(this);
    } finally {
      this.#mounting = false;
      this.#resolveReady();
    }
  }
}

export const defineSvedaChatElement = (): void => {
  if (customElements.get(ELEMENT_TAG)) {
    return;
  }

  customElements.define(ELEMENT_TAG, SvedaChatCustomElement);
};

defineSvedaChatElement();

export {
  ELEMENT_TAG,
  parseSessionAttributes,
  requestHostSession,
  resolveSvedaSession,
};
