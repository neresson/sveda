export type SvedaMergeableChatHistory<TMessage> = {
  id: string;
  title: string;
  preview?: string;
  messages: TMessage[];
  tokensUsed: number;
  contextWindowTokens: number;
  createdAt: number;
  updatedAt: number;
  messagesLoaded: boolean;
};

export const mergeServerChatHistories = <T extends SvedaMergeableChatHistory<unknown>>(
  serverHistories: T[],
  currentChat: T | null
): T[] => {
  if (!currentChat) {
    return serverHistories;
  }

  const index = serverHistories.findIndex(chat => chat.id === currentChat.id);
  if (index === -1) {
    return [currentChat, ...serverHistories];
  }

  if (currentChat.messagesLoaded && !serverHistories[index].messagesLoaded) {
    const next = [...serverHistories];
    next[index] = {
      ...serverHistories[index],
      messages: currentChat.messages,
      messagesLoaded: true,
    };
    return next;
  }

  return serverHistories;
};

export const resolveMergedCurrentChat = <T extends { id: string }>(
  histories: T[],
  currentChat: T | null
): T | null => {
  if (!currentChat) {
    return null;
  }

  return histories.find(chat => chat.id === currentChat.id) ?? currentChat;
};

export const upsertChatHistoryMessages = <T extends SvedaMergeableChatHistory<unknown>>(
  histories: T[],
  currentChat: T | null,
  chatId: string,
  messages: T['messages'],
  now: number = Date.now()
): { histories: T[]; current: T | null } => {
  const index = histories.findIndex(chat => chat.id === chatId);
  if (index !== -1) {
    const next = [...histories];
    next[index] = {
      ...histories[index],
      messages,
      messagesLoaded: true,
      updatedAt: now,
    };

    return {
      histories: next,
      current: currentChat?.id === chatId ? next[index] : currentChat,
    };
  }

  const base = currentChat?.id === chatId ? currentChat : null;
  const upserted = {
    ...(base ?? {
      id: chatId,
      title: '',
      preview: '',
      tokensUsed: 0,
      contextWindowTokens: 0,
      createdAt: now,
    }),
    id: chatId,
    messages,
    messagesLoaded: true,
    updatedAt: now,
  } as T;

  return {
    histories: [upserted, ...histories],
    current: currentChat === null || currentChat.id === chatId ? upserted : currentChat,
  };
};
