import { describe, expect, it } from 'vitest';
import {
  mergeServerChatHistories,
  resolveMergedCurrentChat,
  upsertChatHistoryMessages,
  type SvedaMergeableChatHistory,
} from '../src/lib/chatHistoryMerge';

type TestChat = SvedaMergeableChatHistory<{ role: string; content?: string }>;

const chat = (id: string, overrides: Partial<TestChat> = {}): TestChat => ({
  id,
  title: '',
  messages: [],
  tokensUsed: 0,
  contextWindowTokens: 0,
  createdAt: 1,
  updatedAt: 1,
  messagesLoaded: false,
  ...overrides,
});

describe('mergeServerChatHistories', () => {
  it('keeps a local current chat that the server list does not yet include', () => {
    const current = chat('chat_local', {
      messagesLoaded: true,
      messages: [{ role: 'assistant', content: 'welcome' }],
    });

    const merged = mergeServerChatHistories([chat('chat_server')], current);

    expect(merged.map(item => item.id)).toEqual(['chat_local', 'chat_server']);
    expect(resolveMergedCurrentChat(merged, current)?.messages).toEqual(current.messages);
  });

  it('keeps loaded local messages when the server only returns a summary', () => {
    const current = chat('chat_1', {
      messagesLoaded: true,
      messages: [{ role: 'user', content: 'привет' }],
    });

    const merged = mergeServerChatHistories(
      [chat('chat_1', { title: 'From server', messagesLoaded: false })],
      current
    );

    expect(merged[0].messages).toEqual(current.messages);
    expect(merged[0].messagesLoaded).toBe(true);
    expect(merged[0].title).toBe('From server');
  });
});

describe('upsertChatHistoryMessages', () => {
  it('updates a detached current chat that is missing from the list', () => {
    const current = chat('chat_local', {
      messagesLoaded: true,
      messages: [{ role: 'assistant', content: 'welcome' }],
    });
    const messages = [
      { role: 'user', content: 'привет' },
      { role: 'assistant', content: 'Здравствуйте' },
    ];

    const result = upsertChatHistoryMessages([], current, 'chat_local', messages, 42);

    expect(result.histories).toHaveLength(1);
    expect(result.current?.messages).toEqual(messages);
    expect(result.current?.messagesLoaded).toBe(true);
    expect(result.current?.updatedAt).toBe(42);
  });

  it('replaces messages when the chat is already in the list', () => {
    const existing = chat('chat_1', {
      messagesLoaded: true,
      messages: [{ role: 'assistant', content: 'welcome' }],
    });
    const messages = [{ role: 'user', content: 'привет' }];

    const result = upsertChatHistoryMessages([existing], existing, 'chat_1', messages, 9);

    expect(result.histories[0].messages).toEqual(messages);
    expect(result.current?.messages).toEqual(messages);
  });
});
