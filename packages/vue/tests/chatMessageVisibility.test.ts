import { describe, expect, it } from 'vitest';
import {
  assistantHasActiveTools,
  shouldShowChatPendingIndicator,
} from '../src/lib/chatMessageVisibility';

describe('shouldShowChatPendingIndicator', () => {
  it('stays visible while the assistant is only calling tools', () => {
    expect(
      shouldShowChatPendingIndicator(true, [
        { role: 'user', content: 'find water articles' },
        {
          role: 'assistant',
          parts: [{ type: 'tool-call', toolCallId: 't1', toolName: 'web_search' }],
        },
      ])
    ).toBe(true);
  });

  it('hides once the answer text starts', () => {
    expect(
      shouldShowChatPendingIndicator(true, [
        {
          role: 'assistant',
          parts: [
            { type: 'tool-result', toolCallId: 't1', toolName: 'web_search', output: 'ok' },
            { type: 'text', text: 'Here are a few sources.' },
          ],
        },
      ])
    ).toBe(false);
  });

  it('does not treat finished tools as still running', () => {
    expect(
      assistantHasActiveTools({
        role: 'assistant',
        parts: [
          { type: 'tool-call', toolCallId: 't1', toolName: 'web_search' },
          { type: 'tool-result', toolCallId: 't1', toolName: 'web_search', output: 'ok' },
        ],
      })
    ).toBe(false);

    expect(
      assistantHasActiveTools({
        role: 'assistant',
        parts: [{ type: 'tool-call', toolCallId: 't1', toolName: 'web_search' }],
      })
    ).toBe(true);
  });
});
