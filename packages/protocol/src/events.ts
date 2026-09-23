export const SVEDA_PROTOCOL_VERSION = '1.0' as const;

export const SVEDA_STREAM_EVENTS = [
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

export type SvedaStreamEventType = (typeof SVEDA_STREAM_EVENTS)[number];

export type SvedaToolTarget = 'backend' | 'frontend';

export type SvedaRenderHint =
  | 'confirm_dialog'
  | 'resource_links'
  | 'plain'
  | 'custom';

export interface SvedaStreamEventBase {
  type: SvedaStreamEventType;
  chatId?: string;
  messageId?: string;
  timestamp?: string;
}

export interface SvedaMessageStartEvent extends SvedaStreamEventBase {
  type: 'message.start';
}

export interface SvedaTextDeltaEvent extends SvedaStreamEventBase {
  type: 'text.delta';
  delta: string;
}

export interface SvedaReasoningDeltaEvent extends SvedaStreamEventBase {
  type: 'reasoning.delta';
  delta: string;
}

export interface SvedaToolCallEvent extends SvedaStreamEventBase {
  type: 'tool.call';
  toolCallId: string;
  toolName: string;
  target: SvedaToolTarget;
  input: Record<string, unknown>;
  confirmation?: 'required';
}

export interface SvedaToolResultEvent extends SvedaStreamEventBase {
  type: 'tool.result';
  toolCallId: string;
  toolName: string;
  output: unknown;
  renderHint?: SvedaRenderHint;
  renderData?: Record<string, unknown>;
}

export interface SvedaToolProgressTask {
  id: string;
  label: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  detail?: string;
}

export interface SvedaToolProgressEvent extends SvedaStreamEventBase {
  type: 'tool.progress';
  phase?: string;
  tasks: SvedaToolProgressTask[];
}

export interface SvedaContextUsageEvent extends SvedaStreamEventBase {
  type: 'context.usage';
  usedTokens: number;
  maxTokens: number;
  percent: number;
}

export interface SvedaChatTitleEvent extends SvedaStreamEventBase {
  type: 'chat.title';
  title: string;
}

export interface SvedaMaxStepsEvent extends SvedaStreamEventBase {
  type: 'max_steps';
  maxSteps: number;
  canContinue: boolean;
}

export interface SvedaMessageEndEvent extends SvedaStreamEventBase {
  type: 'message.end';
  finishReason?: string;
  usage?: {
    promptTokens?: number;
    completionTokens?: number;
    totalTokens?: number;
  };
}

export interface SvedaErrorEvent extends SvedaStreamEventBase {
  type: 'error';
  code: string;
  message: string;
}

export type SvedaStreamEvent =
  | SvedaMessageStartEvent
  | SvedaTextDeltaEvent
  | SvedaReasoningDeltaEvent
  | SvedaToolCallEvent
  | SvedaToolResultEvent
  | SvedaToolProgressEvent
  | SvedaContextUsageEvent
  | SvedaChatTitleEvent
  | SvedaMaxStepsEvent
  | SvedaMessageEndEvent
  | SvedaErrorEvent;

export interface SvedaClientToolDefinition {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  confirmation?: 'required';
}

export interface SvedaToolDecision {
  toolCallId: string;
  decision: 'approve' | 'deny';
}

export interface SvedaChatMessage {
  id?: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content?: string;
  parts?: Array<Record<string, unknown>>;
}

export interface SvedaStreamRequest {
  messages: SvedaChatMessage[];
  prompt?: string;
  chatId?: string;
  context?: Record<string, unknown>;
  clientTools?: SvedaClientToolDefinition[];
  toolDecisions?: SvedaToolDecision[];
  model?: string;
  provider?: string;
  options?: Record<string, unknown>;
}

export function isSvedaStreamEvent(value: unknown): value is SvedaStreamEvent {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const type = (value as { type?: unknown }).type;
  return typeof type === 'string' && (SVEDA_STREAM_EVENTS as readonly string[]).includes(type);
}

export function parseSvedaStreamLine(line: string): SvedaStreamEvent | null {
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
    if (isSvedaStreamEvent(parsed)) {
      return parsed;
    }

    return null;
  } catch {
    return null;
  }
}
