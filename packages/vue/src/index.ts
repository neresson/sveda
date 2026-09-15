export * from '@veda-ai/core';

export {
  default,
  createVeda,
  useVedaClient,
  useVedaConfig,
  VedaClientKey,
  VedaConfigKey,
} from './plugin';
export type {
  VedaPluginOptions,
  VedaPlugin,
  VedaConfig,
  VedaBrand,
  VedaModelOption,
  VedaQuickPrompt,
} from './plugin';

export {
  applyVedaAppearance,
  buildAppearanceCss,
  formatRadiusPx,
  hexToHsl,
  hslToHex,
  isVedaHsl,
  parseRadiusPx,
  resolveVedaAppearance,
  sanitizeVedaLauncher,
  sanitizeVedaLauncherImage,
  sanitizeVedaRadius,
  sanitizeVedaTheme,
  useVedaLauncher,
  VEDA_APPEARANCE_PRESET_IDS,
  VEDA_APPEARANCE_PRESETS,
  VEDA_APPEARANCE_STYLE_ID,
  VEDA_DEFAULT_LAUNCHER_ICON,
  VEDA_LAUNCHER_ICON_IDS,
  VEDA_LAUNCHER_IMAGE_MAX_BYTES,
  VEDA_LAUNCHER_IMAGE_MAX_CHARS,
  VEDA_TOKEN_KEYS,
} from './appearance';
export type {
  VedaAppearance,
  VedaAppearanceLauncher,
  VedaAppearancePresetId,
  VedaAppearanceTheme,
  VedaAppearanceTokens,
  VedaLauncherIconId,
  VedaTokenKey,
} from './appearance';
export { vedaLauncherIconComponent, vedaLauncherIconMap } from './launcherIcons';

export { createVedaI18n, installVedaI18n, useVedaT, VedaI18nKey } from './i18n/index';
export type { VedaI18n, VedaI18nOptions, VedaMessages } from './i18n/index';

export { default as VedaChat } from './components/shell/VedaChat.vue';
export { default as Toaster } from './ui/toast/Toaster.vue';
export { default as VedaToolbar } from './components/shell/VedaToolbar.vue';
export { default as VedaTabs } from './components/shell/VedaTabs.vue';
export { default as VedaLandingView } from './components/shell/VedaLandingView.vue';
export { default as VedaInputSection } from './components/shell/VedaInputSection.vue';
export { default as VedaModelControls } from './components/shell/VedaModelControls.vue';
export { default as VedaModelSelect } from './components/shell/VedaModelSelect.vue';
export { default as VedaQuickPrompts } from './components/shell/VedaQuickPrompts.vue';
export { default as VedaHistorySheet } from './components/shell/VedaHistorySheet.vue';
export { default as VedaHistorySidebar } from './components/shell/VedaHistorySidebar.vue';
export { default as VedaAgentTasksPanel } from './components/shell/VedaAgentTasksPanel.vue';
export { default as VedaAgentCompletedNotice } from './components/shell/VedaAgentCompletedNotice.vue';
export { default as VedaMaxStepsNotice } from './components/shell/VedaMaxStepsNotice.vue';
export { default as VedaContextUsageBar } from './components/shell/VedaContextUsageBar.vue';
export { default as VedaResizeHandles } from './components/shell/VedaResizeHandles.vue';
export { default as VedaMinimizedTrigger } from './components/shell/VedaMinimizedTrigger.vue';

export {
  applyVedaAppearance,
  buildAppearanceCss,
  formatRadiusPx,
  hexToHsl,
  hslToHex,
  parseRadiusPx,
  resolveVedaAppearance,
  sanitizeVedaLauncher,
  sanitizeVedaLauncherImage,
  sanitizeVedaRadius,
  sanitizeVedaTheme,
  useVedaLauncher,
  vedaLauncher,
  VEDA_APPEARANCE_PRESET_IDS,
  VEDA_APPEARANCE_PRESETS,
  VEDA_APPEARANCE_STYLE_ID,
  VEDA_DEFAULT_LAUNCHER_ICON,
  VEDA_LAUNCHER_ICON_IDS,
  VEDA_LAUNCHER_IMAGE_MAX_BYTES,
  VEDA_LAUNCHER_IMAGE_MAX_CHARS,
  VEDA_TOKEN_KEYS,
} from './appearance';
export type {
  VedaAppearance,
  VedaAppearanceLauncher,
  VedaAppearancePresetId,
  VedaAppearanceTheme,
  VedaAppearanceTokens,
  VedaLauncherIconId,
  VedaTokenKey,
} from './appearance';
export { vedaLauncherIconComponent, vedaLauncherIconMap } from './launcherIcons';

export { useVedaChat } from './composables/useVedaChat';
export type { VedaChatHistory, VedaChatState } from './composables/useVedaChat';
export { useVedaStreaming } from './composables/useVedaStreaming';
export type {
  VedaStreamingStore,
  VedaStreamingOptions,
  VedaSendPayload,
} from './composables/useVedaStreaming';
export {
  useVedaChatLayout,
  VEDA_CHAT_LAYOUT_MIN_WIDTH,
  VEDA_CHAT_LAYOUT_MIN_HEIGHT,
  VEDA_CHAT_LAYOUT_MAX_WIDTH,
  VEDA_CHAT_LAYOUT_MAX_HEIGHT,
  VEDA_CHAT_MAIN_CONTENT_MIN_WIDTH,
} from './composables/useVedaChatLayout';
export type { VedaChatViewMode } from './composables/useVedaChatLayout';
export { useVedaAgentTasks } from './composables/useVedaAgentTasks';
export type {
  VedaAgentTaskItem,
  VedaAgentTasksState,
  VedaAgentTasksPayload,
} from './composables/useVedaAgentTasks';
export { useVedaChatPage } from './composables/useVedaChatPage';
export type { VedaChatPageOptions, VedaChatPageRefs } from './composables/useVedaChatPage';
export { useVedaMessaging } from './composables/useVedaMessaging';
export { useVedaDocuments } from './composables/useVedaDocuments';
export type { VedaExtractedDocumentItem } from './composables/useVedaDocuments';
export { useVedaModel } from './composables/useVedaModel';
export type { VedaChatModelOption } from './composables/useVedaModel';
export { useVedaMaxStepsContinue } from './composables/useVedaMaxStepsContinue';
export type { VedaMaxStepsReachedData } from './composables/useVedaMaxStepsContinue';
export { useVedaPrompts } from './composables/useVedaPrompts';
export type { VedaQuickPromptItem } from './composables/useVedaPrompts';
export { useVedaScroll } from './composables/useVedaScroll';
export type { VedaScrollContainer } from './composables/useVedaScroll';
export { useVedaShell } from './composables/useVedaShell';
export type { VedaBrandInfo } from './composables/useVedaShell';
export { useVedaContextWindow, VEDA_CHAT_MAX_CONTEXT_TOKENS } from './composables/useVedaContextWindow';
export { useVedaBootstrap } from './composables/useVedaBootstrap';
export { useVedaReadable } from './composables/useVedaReadable';
export { useVedaTool } from './composables/useVedaTool';
export type { UseVedaToolOptions } from './composables/useVedaTool';

export { isNewChatPlaceholder, buildChatTabTitleMap } from './lib/chatTabs';
export type { VedaTabChat } from './lib/chatTabs';
export { isChatTitlePlaceholder, deriveProvisionalChatTitle } from './lib/chatTitle';
export {
  VEDA_CHAT_MINIMIZED_STORAGE_KEY,
  readPersistedMinimized,
  writePersistedMinimized,
} from './lib/chatUiStorage';
export { humanizeModelId, getVedaModelDisplayName } from './lib/modelLabels';
export { finalizeMessagesForDisplay } from './lib/finalizeMessages';
export type { VedaFinalizableMessage } from './lib/finalizeMessages';
export {
  getUserMessageText,
  getUserMessageAttachmentNames,
  userMessageHasVisibleContent,
} from './lib/userMessage';
export type { VedaUserMessageLike } from './lib/userMessage';

export const VEDA_VUE_VERSION = '0.1.0';
