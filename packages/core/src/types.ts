import type { SvedaRenderHint, SvedaToolTarget } from '@sveda-ai/protocol';

export type SvedaMessageRole = 'user' | 'assistant' | 'system' | 'tool';

export type SvedaMessagePart =
  | { type: 'text'; text: string }
  | { type: 'reasoning'; text: string }
  | {
      type: 'tool-call';
      toolCallId: string;
      toolName: string;
      target: SvedaToolTarget;
      input: Record<string, unknown>;
      confirmation?: 'required';
    }
  | {
      type: 'tool-result';
      toolCallId: string;
      toolName: string;
      output: unknown;
      renderHint?: SvedaRenderHint;
      renderData?: Record<string, unknown>;
    }
  | { type: 'file'; name: string; mediaType?: string; url?: string };

export interface SvedaDisplayMessage {
  id: string;
  role: SvedaMessageRole;
  parts: SvedaMessagePart[];
  createdAt?: string;
}

export type SvedaSessionStatus = 'idle' | 'submitted' | 'streaming' | 'error';

export interface SvedaChatHistorySummary {
  chatId: string;
  title: string;
  updatedAt?: string;
  [key: string]: unknown;
}

export interface SvedaChatHistoryDetail extends SvedaChatHistorySummary {
  messages: SvedaDisplayMessage[];
}

export function messageText(message: SvedaDisplayMessage): string {
  return message.parts
    .filter((part): part is Extract<SvedaMessagePart, { type: 'text' }> => part.type === 'text')
    .map(part => part.text)
    .join('');
}

export function createMessageId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }

  return `msg_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;
}
