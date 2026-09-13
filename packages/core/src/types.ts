import type { VedaRenderHint, VedaToolTarget } from '@veda-ai/protocol';

export type VedaMessageRole = 'user' | 'assistant' | 'system' | 'tool';

export type VedaMessagePart =
  | { type: 'text'; text: string }
  | { type: 'reasoning'; text: string }
  | {
      type: 'tool-call';
      toolCallId: string;
      toolName: string;
      target: VedaToolTarget;
      input: Record<string, unknown>;
    }
  | {
      type: 'tool-result';
      toolCallId: string;
      toolName: string;
      output: unknown;
      renderHint?: VedaRenderHint;
      renderData?: Record<string, unknown>;
    }
  | { type: 'file'; name: string; mediaType?: string; url?: string };

export interface VedaDisplayMessage {
  id: string;
  role: VedaMessageRole;
  parts: VedaMessagePart[];
  createdAt?: string;
}

export type VedaSessionStatus = 'idle' | 'submitted' | 'streaming' | 'error';

export interface VedaChatHistorySummary {
  chatId: string;
  title: string;
  updatedAt?: string;
  [key: string]: unknown;
}

export interface VedaChatHistoryDetail extends VedaChatHistorySummary {
  messages: VedaDisplayMessage[];
}

export function messageText(message: VedaDisplayMessage): string {
  return message.parts
    .filter((part): part is Extract<VedaMessagePart, { type: 'text' }> => part.type === 'text')
    .map(part => part.text)
    .join('');
}

export function createMessageId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }

  return `msg_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;
}
