export * from './appearance.js';
export * from './chatStore.js';
export * from './streaming.js';
export * from './i18n/catalogs.js';
export type { SvedaMessages } from './i18n/types.js';

export {
  mergeServerChatHistories,
  resolveMergedCurrentChat,
  upsertChatHistoryMessages,
} from './lib/chatHistoryMerge.js';
export type { SvedaMergeableChatHistory } from './lib/chatHistoryMerge.js';

export { renderMarkdown, injectResourceLinks } from './lib/markdown.js';
export type { SvedaMarkdownOptions } from './lib/markdown.js';

export { hasPendingToolConfirmation } from './lib/toolConfirmation.js';

export {
  SVEDA_CHAT_MINIMIZED_STORAGE_KEY,
  readPersistedMinimized,
  readPersistedMinimizedPreference,
  isPersistedSessionChatOpen,
  writePersistedMinimized,
} from './lib/chatUiStorage.js';

export { isChatTitlePlaceholder, deriveProvisionalChatTitle } from './lib/chatTitle.js';

export {
  messageHasVisibleAssistantContent,
  messageHasAssistantAnswerText,
  assistantHasActiveTools,
  shouldShowChatPendingIndicator,
} from './lib/chatMessageVisibility.js';

export {
  getUserMessageText,
  getUserMessageAttachmentNames,
  userMessageHasVisibleContent,
} from './lib/userMessage.js';
export type { SvedaUserMessageLike } from './lib/userMessage.js';

export { humanizeModelId, getSvedaModelDisplayName } from './lib/modelLabels.js';

export { finalizeMessagesForDisplay } from './lib/finalizeMessages.js';
export type { SvedaFinalizableMessage } from './lib/finalizeMessages.js';

export { isNewChatPlaceholder, buildChatTabTitleMap } from './lib/chatTabs.js';
export type { SvedaTabChat } from './lib/chatTabs.js';

export {
  SVEDA_CHAT_LAYOUT_MIN_WIDTH,
  SVEDA_CHAT_LAYOUT_MIN_HEIGHT,
  SVEDA_CHAT_LAYOUT_MAX_WIDTH,
  SVEDA_CHAT_LAYOUT_MAX_HEIGHT,
  SVEDA_HISTORY_SIDEBAR_WIDTH,
  resolveEmbedHostSize,
  pointerScreenDelta,
  applyFloatingResize,
  applyFixedLeftResize,
} from './lib/chatResize.js';
export type { EmbedHostSize, ChatBoxSize } from './lib/chatResize.js';

export { chatComponentI18nKeys } from './lib/chatI18nKeys.js';

export {
  isAssistantToolPart,
  buildAssistantMessageSegments,
  getToolNameFromPart,
  parseToolResultOutput,
  getActivityGroupKey,
  getActivityGroupLabel,
  collectResourceLinks,
  enrichMessagesWithResourceLinks,
} from './lib/assistantMessage.js';
export type {
  SvedaResourceLink,
  SvedaAssistantMessagePart,
  SvedaLegacyActivity,
  SvedaChatMessageLike,
  AssistantMessageSegment,
} from './lib/assistantMessage.js';

export const SVEDA_CHAT_VERSION = '0.3.1';
