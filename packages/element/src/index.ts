export { defineVedaChatElement, VedaChatElement } from './veda-chat-element';
export type {
  VedaChatElementConfig,
  VedaChatElementRootApi,
  VedaChatElementTheme,
} from './veda-chat-element';

export { connectVedaEmbed } from './host-sdk';
export type { ConnectVedaEmbedOptions, VedaEmbedConnection } from './host-sdk';

export { initVedaEmbedHost } from './embed-side';
export type {
  VedaEmbedHost,
  VedaEmbedHostHandlers,
  VedaEmbedHostOptions,
} from './embed-side';

export {
  createVedaEmbedProtocol,
  VEDA_EMBED_SOURCE,
  VEDA_EMBED_VERSION,
} from './embed-protocol';
export type {
  VedaEmbedAckPayload,
  VedaEmbedEnvelope,
  VedaEmbedErrorPayload,
  VedaEmbedEventType,
  VedaEmbedHostCommandType,
  VedaEmbedNavigatePayload,
  VedaEmbedProtocol,
  VedaEmbedReadyPayload,
  VedaEmbedResizePayload,
  VedaEmbedSendMessagePayload,
  VedaEmbedSetAuthTokenPayload,
  VedaEmbedSetContextPayload,
  VedaEmbedSetLocalePayload,
  VedaEmbedSetThemePayload,
  VedaEmbedTheme,
  VedaEmbedToolProgressPayload,
} from './embed-protocol';

export const VEDA_ELEMENT_VERSION = '0.1.0';
