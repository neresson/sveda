export * from '@sveda-ai/core';

export {
  SvedaProvider,
  createSveda,
  useSvedaClient,
  useSvedaConfig,
  useSvedaContext,
  SvedaContext,
  SVEDA_EMBED_AUTH_EVENT,
} from './provider';
export type {
  SvedaProviderOptions,
  SvedaConfig,
  SvedaBrand,
  SvedaModelOption,
  SvedaQuickPrompt,
  SvedaBeforeSend,
  SvedaContextValue,
} from './provider';

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
} from './appearance';
export type {
  SvedaAppearance,
  SvedaAppearanceChrome,
  SvedaAppearanceLauncher,
  SvedaAppearancePresetId,
  SvedaAppearanceTheme,
  SvedaAppearanceTokens,
  SvedaLauncherIconId,
  SvedaTokenKey,
} from './appearance';

export { svedaLauncherIconComponent, svedaLauncherIconMap } from './launcherIcons';

export { createSvedaI18n, useSvedaT, SvedaI18nContext } from './i18n/index';
export type { SvedaI18n, SvedaI18nOptions, SvedaMessages } from './i18n/index';

export { SvedaChat } from './components/SvedaChat';
export { SvedaMinimizedTrigger } from './components/SvedaMinimizedTrigger';
export { SvedaTabs } from './components/SvedaTabs';
export { SvedaToolbar } from './components/SvedaToolbar';
export { SvedaComposer } from './components/SvedaComposer';
export { SvedaWelcome } from './components/SvedaWelcome';
export { SvedaMessageList } from './components/SvedaMessageList';
export { SvedaQuickPrompts } from './components/SvedaQuickPrompts';

export { useSvedaChat } from './hooks/useSvedaChat';
export type { SvedaChatHistory, SvedaChatState } from './hooks/useSvedaChat';
export { useSvedaStreaming } from './hooks/useSvedaStreaming';
export type {
  SvedaStreamingStore,
  SvedaStreamingOptions,
  SvedaSendPayload,
} from './hooks/useSvedaStreaming';
export { useSvedaChatPage } from './hooks/useSvedaChatPage';
export type {
  SvedaChatPageOptions,
  SvedaAgentTasksPayload,
} from './hooks/useSvedaChatPage';

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

export const SVEDA_SOLID_VERSION = '0.3.1';
