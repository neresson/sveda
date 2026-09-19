export * from '@sveda-ai/core';

export {
  default,
  createSveda,
  useSvedaClient,
  useSvedaConfig,
  SvedaClientKey,
  SvedaConfigKey,
  SvedaHostEmbedKey,
  SvedaFillHostKey,
  SvedaHideLauncherKey,
  SvedaBeforeSendKey,
} from './plugin';
export type {
  SvedaPluginOptions,
  SvedaPlugin,
  SvedaConfig,
  SvedaBrand,
  SvedaModelOption,
  SvedaQuickPrompt,
  SvedaBeforeSend,
} from './plugin';

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

export { createSvedaI18n, installSvedaI18n, useSvedaT, SvedaI18nKey } from './i18n/index';
export type { SvedaI18n, SvedaI18nOptions, SvedaMessages } from './i18n/index';

export { default as SvedaChat } from './components/shell/SvedaChat.vue';
export { default as Toaster } from './ui/toast/Toaster.vue';
export { default as SvedaToolbar } from './components/shell/SvedaToolbar.vue';
export { default as SvedaTabs } from './components/shell/SvedaTabs.vue';
export { default as SvedaLandingView } from './components/shell/SvedaLandingView.vue';
export { default as SvedaInputSection } from './components/shell/SvedaInputSection.vue';
export { default as SvedaModelControls } from './components/shell/SvedaModelControls.vue';
export { default as SvedaModelSelect } from './components/shell/SvedaModelSelect.vue';
export { default as SvedaQuickPrompts } from './components/shell/SvedaQuickPrompts.vue';
export { default as SvedaHistorySheet } from './components/shell/SvedaHistorySheet.vue';
export { default as SvedaHistorySidebar } from './components/shell/SvedaHistorySidebar.vue';
export { default as SvedaAgentTasksPanel } from './components/shell/SvedaAgentTasksPanel.vue';
export { default as SvedaAgentCompletedNotice } from './components/shell/SvedaAgentCompletedNotice.vue';
export { default as SvedaMaxStepsNotice } from './components/shell/SvedaMaxStepsNotice.vue';
export { default as SvedaContextUsageBar } from './components/shell/SvedaContextUsageBar.vue';
export { default as SvedaResizeHandles } from './components/shell/SvedaResizeHandles.vue';
export { default as SvedaMinimizedTrigger } from './components/shell/SvedaMinimizedTrigger.vue';

export { useSvedaChat } from './composables/useSvedaChat';
export type { SvedaChatHistory, SvedaChatState } from './composables/useSvedaChat';
export { useSvedaStreaming } from './composables/useSvedaStreaming';
export type {
  SvedaStreamingStore,
  SvedaStreamingOptions,
  SvedaSendPayload,
} from './composables/useSvedaStreaming';
export {
  useSvedaChatLayout,
  SVEDA_CHAT_LAYOUT_MIN_WIDTH,
  SVEDA_CHAT_LAYOUT_MIN_HEIGHT,
  SVEDA_CHAT_LAYOUT_MAX_WIDTH,
  SVEDA_CHAT_LAYOUT_MAX_HEIGHT,
  SVEDA_CHAT_MAIN_CONTENT_MIN_WIDTH,
  SVEDA_HISTORY_SIDEBAR_WIDTH,
  resolveEmbedHostSize,
} from './composables/useSvedaChatLayout';
export type { SvedaChatViewMode } from './composables/useSvedaChatLayout';
export { useSvedaAgentTasks } from './composables/useSvedaAgentTasks';
export type {
  SvedaAgentTaskItem,
  SvedaAgentTasksState,
  SvedaAgentTasksPayload,
} from './composables/useSvedaAgentTasks';
export { useSvedaChatPage } from './composables/useSvedaChatPage';
export type { SvedaChatPageOptions, SvedaChatPageRefs } from './composables/useSvedaChatPage';
export { useSvedaMessaging } from './composables/useSvedaMessaging';
export { useSvedaDocuments } from './composables/useSvedaDocuments';
export type { SvedaExtractedDocumentItem } from './composables/useSvedaDocuments';
export { useSvedaModel } from './composables/useSvedaModel';
export type { SvedaChatModelOption } from './composables/useSvedaModel';
export { useSvedaMaxStepsContinue } from './composables/useSvedaMaxStepsContinue';
export type { SvedaMaxStepsReachedData } from './composables/useSvedaMaxStepsContinue';
export { useSvedaPrompts } from './composables/useSvedaPrompts';
export type { SvedaQuickPromptItem } from './composables/useSvedaPrompts';
export { useSvedaScroll } from './composables/useSvedaScroll';
export type { SvedaScrollContainer } from './composables/useSvedaScroll';
export { useSvedaShell } from './composables/useSvedaShell';
export type { SvedaBrandInfo } from './composables/useSvedaShell';
export { useSvedaContextWindow, SVEDA_CHAT_MAX_CONTEXT_TOKENS } from './composables/useSvedaContextWindow';
export { useSvedaBootstrap } from './composables/useSvedaBootstrap';
export { useSvedaReadable } from './composables/useSvedaReadable';
export { useSvedaTool } from './composables/useSvedaTool';
export type { UseSvedaToolOptions } from './composables/useSvedaTool';

export { isNewChatPlaceholder, buildChatTabTitleMap } from './lib/chatTabs';
export type { SvedaTabChat } from './lib/chatTabs';
export { isChatTitlePlaceholder, deriveProvisionalChatTitle } from './lib/chatTitle';
export {
  SVEDA_CHAT_MINIMIZED_STORAGE_KEY,
  readPersistedMinimized,
  readPersistedMinimizedPreference,
  isPersistedSessionChatOpen,
  writePersistedMinimized,
} from './lib/chatUiStorage';
export { humanizeModelId, getSvedaModelDisplayName } from './lib/modelLabels';
export { finalizeMessagesForDisplay } from './lib/finalizeMessages';
export type { SvedaFinalizableMessage } from './lib/finalizeMessages';
export {
  getUserMessageText,
  getUserMessageAttachmentNames,
  userMessageHasVisibleContent,
} from './lib/userMessage';
export type { SvedaUserMessageLike } from './lib/userMessage';

export const SVEDA_VUE_VERSION = '0.1.0';
