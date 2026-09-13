export const VEDA_PROTOCOL_VERSION = '1.0' as const;

export const VEDA_STREAM_EVENTS = [
  'message.start',
  'text.delta',
  'reasoning.delta',
  'tool.call',
  'tool.result',
  'tool.progress',
  'context.usage',
  'chat.title',
  'max_steps',
  'message.end',
  'error',
] as const;

export type VedaStreamEventType = (typeof VEDA_STREAM_EVENTS)[number];

export type VedaToolTarget = 'backend' | 'frontend';

export type VedaRenderHint =
  | 'confirm_dialog'
  | 'resource_links'
  | 'plain'
  | 'custom';

export interface VedaStreamEventBase {
  type: VedaStreamEventType;
  chatId?: string;
  messageId?: string;
  timestamp?: string;
}

export interface VedaMessageStartEvent extends VedaStreamEventBase {
  type: 'message.start';
}

export interface VedaTextDeltaEvent extends VedaStreamEventBase {
  type: 'text.delta';
  delta: string;
}

export interface VedaReasoningDeltaEvent extends VedaStreamEventBase {
  type: 'reasoning.delta';
  delta: string;
}

export interface VedaToolCallEvent extends VedaStreamEventBase {
  type: 'tool.call';
  toolCallId: string;
  toolName: string;
  target: VedaToolTarget;
  input: Record<string, unknown>;
}

export interface VedaToolResultEvent extends VedaStreamEventBase {
  type: 'tool.result';
  toolCallId: string;
  toolName: string;
  output: unknown;
  renderHint?: VedaRenderHint;
  renderData?: Record<string, unknown>;
}

export interface VedaToolProgressTask {
  id: string;
  label: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  detail?: string;
}

export interface VedaToolProgressEvent extends VedaStreamEventBase {
  type: 'tool.progress';
  phase?: string;
  tasks: VedaToolProgressTask[];
}

export interface VedaContextUsageEvent extends VedaStreamEventBase {
  type: 'context.usage';
  usedTokens: number;
  maxTokens: number;
  percent: number;
}

export interface VedaChatTitleEvent extends VedaStreamEventBase {
  type: 'chat.title';
  title: string;
}

export interface VedaMaxStepsEvent extends VedaStreamEventBase {
  type: 'max_steps';
  maxSteps: number;
  canContinue: boolean;
}

export interface VedaMessageEndEvent extends VedaStreamEventBase {
  type: 'message.end';
  finishReason?: string;
  usage?: {
    promptTokens?: number;
    completionTokens?: number;
    totalTokens?: number;
  };
}

export interface VedaErrorEvent extends VedaStreamEventBase {
  type: 'error';
  code: string;
  message: string;
}

export type VedaStreamEvent =
  | VedaMessageStartEvent
  | VedaTextDeltaEvent
  | VedaReasoningDeltaEvent
  | VedaToolCallEvent
  | VedaToolResultEvent
  | VedaToolProgressEvent
  | VedaContextUsageEvent
  | VedaChatTitleEvent
  | VedaMaxStepsEvent
  | VedaMessageEndEvent
  | VedaErrorEvent;

export interface VedaClientToolDefinition {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}

export interface VedaChatMessage {
  id?: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content?: string;
  parts?: Array<Record<string, unknown>>;
}

export interface VedaStreamRequest {
  messages: VedaChatMessage[];
  chatId?: string;
  context?: Record<string, unknown>;
  clientTools?: VedaClientToolDefinition[];
  model?: string;
  provider?: string;
  options?: Record<string, unknown>;
}

export function isVedaStreamEvent(value: unknown): value is VedaStreamEvent {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const type = (value as { type?: unknown }).type;
  return typeof type === 'string' && (VEDA_STREAM_EVENTS as readonly string[]).includes(type);
}

export function parseVedaStreamLine(line: string): VedaStreamEvent | null {
  const trimmed = line.trim();
  if (!trimmed.startsWith('data:')) {
    return null;
  }

  const payload = trimmed.slice(5).trim();
  if (!payload || payload === '[DONE]') {
    return null;
  }

  try {
    const parsed = JSON.parse(payload) as unknown;
    if (isVedaStreamEvent(parsed)) {
      return parsed;
    }

    return null;
  } catch {
    return null;
  }
}
