import { describe, expect, it } from 'vitest';
import { hasPendingToolConfirmation } from '../src/lib/toolConfirmation.js';

describe('hasPendingToolConfirmation', () => {
  it('is true while a required tool call has no result', () => {
    expect(
      hasPendingToolConfirmation([
        {
          parts: [
            { type: 'tool-call', toolCallId: 'tc1', confirmation: 'required' },
          ],
        },
      ])
    ).toBe(true);
  });

  it('is false after the matching tool result arrives', () => {
    expect(
      hasPendingToolConfirmation([
        {
          parts: [
            { type: 'tool-call', toolCallId: 'tc1', confirmation: 'required' },
            { type: 'tool-result', toolCallId: 'tc1' },
          ],
        },
      ])
    ).toBe(false);
  });

  it('ignores tool calls that do not require confirmation', () => {
    expect(
      hasPendingToolConfirmation([
        {
          parts: [{ type: 'tool-call', toolCallId: 'tc1' }],
        },
      ])
    ).toBe(false);
  });
});
