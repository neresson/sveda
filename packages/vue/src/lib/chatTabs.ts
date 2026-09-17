import { userMessageHasVisibleContent, type SvedaUserMessageLike } from './userMessage';

export type SvedaTabChat = {
  id: string;
  title?: string;
  messages?: SvedaUserMessageLike[];
};

export const isNewChatPlaceholder = (chat: SvedaTabChat, newChatLabel: string): boolean => {
  const messages = chat.messages || [];
  if (messages.some(userMessageHasVisibleContent)) {
    return false;
  }

  const title = (chat.title || '').trim();
  if (!title || title === '__NEW_CHAT__' || title === 'New Chat') {
    return true;
  }

  if (title === newChatLabel) {
    return true;
  }

  if (title.startsWith(`${newChatLabel} `)) {
    const suffix = title.slice(newChatLabel.length).trim();
    return /^\d+$/.test(suffix);
  }

  return false;
};

export const buildChatTabTitleMap = (
  tabs: SvedaTabChat[],
  translate: (key: string, params?: Record<string, unknown>) => string
): Map<string, string> => {
  const newChatLabel = translate('newChat');
  const placeholderTabs = tabs.filter(tab => isNewChatPlaceholder(tab, newChatLabel));
  const titles = new Map<string, string>();

  tabs.forEach(tab => {
    if (!isNewChatPlaceholder(tab, newChatLabel)) {
      titles.set(tab.id, (tab.title || '').trim() || newChatLabel);
      return;
    }

    const index = placeholderTabs.findIndex(item => item.id === tab.id) + 1;
    titles.set(
      tab.id,
      placeholderTabs.length > 1 ? translate('newChatNumbered', { number: index }) : newChatLabel
    );
  });

  return titles;
};
