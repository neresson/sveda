export * from '@sveda-ai/core';

export {
  SvedaProvider,
  useSvedaClient,
  useSvedaConfig,
  useSvedaContext,
  useSvedaT,
  createSvedaContextValue,
  SvedaContext,
} from './provider';
export type { SvedaProviderProps } from './provider';

export type {
  SvedaProviderOptions,
  SvedaConfig,
  SvedaBrand,
  SvedaModelOption,
  SvedaQuickPrompt,
  SvedaBeforeSend,
  SvedaContextValue,
} from './types';
export { SVEDA_EMBED_AUTH_EVENT, DEFAULT_CONFIG } from './types';

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
  getSvedaLauncherSnapshot,
  getSvedaChromeSnapshot,
  subscribeSvedaAppearance,
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

export { createSvedaI18n } from './i18n';
export type { SvedaI18n, SvedaI18nOptions, SvedaMessages } from './i18n';

export { SvedaChat } from './components/SvedaChat';
export type { SvedaChatProps } from './components/SvedaChat';
export { SvedaToolbar } from './components/shell/SvedaToolbar';
export { SvedaTabs } from './components/shell/SvedaTabs';
export { SvedaComposer } from './components/shell/SvedaComposer';
export { SvedaMinimizedTrigger } from './components/shell/SvedaMinimizedTrigger';
export { ChatMessageList } from './components/shell/ChatMessageList';

export { useSvedaChat } from './hooks/useSvedaChat';
export type { SvedaChatHistory, SvedaChatState } from './hooks/useSvedaChat';
export { useSvedaStreaming } from './hooks/useSvedaStreaming';
export type {
  SvedaStreamingStore,
  SvedaStreamingOptions,
  SvedaSendPayload,
} from './hooks/useSvedaStreaming';

export {
  enMessages,
  ruMessages,
  chatComponentI18nKeys,
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

export const SVEDA_REACT_VERSION = '0.3.1';
