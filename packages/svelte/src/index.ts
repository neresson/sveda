export * from '@sveda-ai/core';

export {
  createSveda,
  SVEDA_CONTEXT_KEY,
  SVEDA_EMBED_AUTH_EVENT,
  getDefaultSvedaConfig,
} from './provider.js';
export type {
  SvedaPluginOptions,
  SvedaContext,
  SvedaConfig,
  SvedaBrand,
  SvedaModelOption,
  SvedaQuickPrompt,
  SvedaBeforeSend,
} from './provider.js';

export {
  setSvedaContext,
  createSvedaContext,
  useSvedaContext,
  useSvedaClient,
  useSvedaConfig,
  useSvedaT,
} from './context.js';

export {
  applySvedaAppearance,
  buildAppearanceCss,
  formatRadiusPx,
  hexToHsl,
  hslToHex,
  isSvedaAppearanceProvided,
  isSvedaHsl,
  mergeSvedaAppearance,
  parseRadiusPx,
  resolveSvedaAppearance,
  sanitizeSvedaChrome,
  sanitizeSvedaLauncher,
  sanitizeSvedaLauncherImage,
  sanitizeSvedaRadius,
  sanitizeSvedaTheme,
  useSvedaChrome,
  useSvedaLauncher,
  svedaChrome,
  svedaLauncher,
  SVEDA_APPEARANCE_PRESET_IDS,
  SVEDA_APPEARANCE_PRESETS,
  SVEDA_APPEARANCE_STYLE_ID,
  SVEDA_DEFAULT_LAUNCHER_ICON,
  SVEDA_LAUNCHER_ICON_IDS,
  SVEDA_LAUNCHER_IMAGE_MAX_BYTES,
  SVEDA_LAUNCHER_IMAGE_MAX_CHARS,
  SVEDA_TOKEN_KEYS,
} from './appearance.js';
export type {
  SvedaAppearance,
  SvedaAppearanceChrome,
  SvedaAppearanceLauncher,
  SvedaAppearancePresetId,
  SvedaAppearanceTheme,
  SvedaAppearanceTokens,
  SvedaLauncherIconId,
  SvedaTokenKey,
} from './appearance.js';

export { createSvedaI18n } from './i18n.js';
export type { SvedaI18n, SvedaI18nOptions, SvedaMessages } from './i18n.js';

export { default as SvedaProvider } from './components/SvedaProvider.svelte';
export { default as SvedaChat } from './components/SvedaChat.svelte';

export { useSvedaChat } from './hooks/useSvedaChat.svelte.js';
export type { SvedaChatHistory, SvedaChatState } from './hooks/useSvedaChat.svelte.js';
export { useSvedaStreaming } from './hooks/useSvedaStreaming.svelte.js';
export type {
  SvedaStreamingStore,
  SvedaStreamingOptions,
  SvedaSendPayload,
} from './hooks/useSvedaStreaming.svelte.js';

export {
  isNewChatPlaceholder,
  buildChatTabTitleMap,
  isChatTitlePlaceholder,
  deriveProvisionalChatTitle,
  SVEDA_CHAT_MINIMIZED_STORAGE_KEY,
  readPersistedMinimized,
  readPersistedMinimizedPreference,
  isPersistedSessionChatOpen,
  writePersistedMinimized,
  humanizeModelId,
  getSvedaModelDisplayName,
  finalizeMessagesForDisplay,
  getUserMessageText,
  getUserMessageAttachmentNames,
  userMessageHasVisibleContent,
} from '@sveda-ai/chat';
export type {
  SvedaTabChat,
  SvedaFinalizableMessage,
  SvedaUserMessageLike,
} from '@sveda-ai/chat';

export const SVEDA_SVELTE_VERSION = '0.3.1';
