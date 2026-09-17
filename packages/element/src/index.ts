import { applySvedaAppearance, createSveda } from '@sveda-ai/vue';
import { createApp } from 'vue';
import './host.css';
import SvedaChatElement from './SvedaChatElement.vue';
import { parseSessionAttributes, requestHostSession, resolveSvedaSession } from './session';

const ELEMENT_TAG = 'sveda-chat';

type SvedaChatApi = {
  open: () => void;
  close: () => void;
  toggle: () => void;
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

const buildModels = (models: unknown) => {
  if (!Array.isArray(models)) {
    return [];
  }

  return models
    .map((model) => {
      const record = model as Record<string, unknown>;
      const id = String(record.id ?? '').trim();
      if (!id) {
        return null;
      }

      return {
        id,
        label: String(record.label ?? id),
        supportsThinking: Boolean(record.supportsThinking ?? record.supports_thinking ?? record.thinking),
      };
    })
    .filter(Boolean);
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

  const hideLauncher = element.hasAttribute('hide-launcher');
  const config = await loadEmbedConfig(session.origin, session.token);
  const appearance = session.appearance ?? (config as { appearance?: Record<string, unknown> }).appearance ?? null;
  if (appearance) {
    applySvedaAppearance(appearance);
  }

  const plugin = createSveda({
    endpoints: chatEndpoints(session.origin),
    protocolMode: 'sveda',
    credentials: 'omit',
    hostEmbed: true,
    hideLauncher,
    headers: () => ({
      'X-Requested-With': 'XMLHttpRequest',
      'X-Sveda-Embed-Token': session.token,
    }),
    locale: document.documentElement.lang || 'ru',
    brand: { name: (appearance as { brand?: { name?: string } })?.brand?.name || 'Sveda' },
    models: buildModels((config as { models?: unknown }).models),
    appearance,
  });

  const app = createApp(SvedaChatElement, {
    session,
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
  };
  Object.assign(element, api);
  element.dataset.svedaMounted = 'true';

  return api;
};

class SvedaChatCustomElement extends HTMLElement {
  ready: Promise<void>;

  #resolveReady: () => void = () => {};

  constructor() {
    super();
    this.ready = new Promise((resolve) => {
      this.#resolveReady = resolve;
    });
  }

  connectedCallback(): void {
    void mountElement(this).finally(() => this.#resolveReady());
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
