import type { SvedaRenderHint, SvedaStreamEvent } from './events.js';

export type VercelDataPart = {
  type: string;
  [key: string]: unknown;
};

export function svedaEventToVercelDataPart(event: SvedaStreamEvent): VercelDataPart | null {
  switch (event.type) {
    case 'text.delta':
      return { type: 'text-delta', textDelta: event.delta };
    case 'reasoning.delta':
      return { type: 'reasoning-delta', reasoningDelta: event.delta };
    case 'tool.call':
      return {
        type: 'tool-call',
        toolCallId: event.toolCallId,
        toolName: event.toolName,
        args: event.input,
        target: event.target,
        confirmation: event.confirmation,
      };
    case 'tool.result':
      return {
        type: 'tool-result',
        toolCallId: event.toolCallId,
        toolName: event.toolName,
        result: event.output,
        renderHint: event.renderHint,
        renderData: event.renderData,
      };
    case 'tool.progress':
      return {
        type: 'data-toolProgress',
        data: { phase: event.phase, tasks: event.tasks },
      };
    case 'context.usage':
      return {
        type: 'data-contextUsage',
        data: {
          usedTokens: event.usedTokens,
          maxTokens: event.maxTokens,
          percent: event.percent,
        },
      };
    case 'chat.title':
      return { type: 'data-chatTitle', data: { title: event.title } };
    case 'max_steps':
      return {
        type: 'data-maxStepsReached',
        data: { maxSteps: event.maxSteps, canContinue: event.canContinue },
      };
    case 'message.end':
      return {
        type: 'finish',
        finishReason: event.finishReason ?? 'stop',
        usage: event.usage,
      };
    case 'error':
      return { type: 'error', error: { code: event.code, message: event.message } };
    default:
      return null;
  }
}

export function vercelDataPartToSvedaEvent(part: VercelDataPart): SvedaStreamEvent | null {
  const data = (part.data ?? {}) as Record<string, unknown>;

  switch (part.type) {
    case 'text-delta':
      return { type: 'text.delta', delta: String(part.textDelta ?? '') };
    case 'reasoning-delta':
      return { type: 'reasoning.delta', delta: String(part.reasoningDelta ?? '') };
    case 'tool-call':
      return {
        type: 'tool.call',
        toolCallId: String(part.toolCallId ?? ''),
        toolName: String(part.toolName ?? ''),
        target: part.target === 'frontend' ? 'frontend' : 'backend',
        input: (part.args as Record<string, unknown>) ?? {},
        ...(part.confirmation === 'required' ? { confirmation: 'required' as const } : {}),
      };
    case 'tool-result':
      return {
        type: 'tool.result',
        toolCallId: String(part.toolCallId ?? ''),
        toolName: String(part.toolName ?? ''),
        output: part.result,
        renderHint: part.renderHint as SvedaRenderHint | undefined,
        renderData: part.renderData as Record<string, unknown> | undefined,
      };
    case 'data-toolProgress': {
      const tasks = Array.isArray(data.tasks) ? data.tasks : [];
      return {
        type: 'tool.progress',
        phase: typeof data.phase === 'string' ? data.phase : undefined,
        tasks: tasks as Array<{
          id: string;
          label: string;
          status: 'pending' | 'running' | 'completed' | 'failed';
          detail?: string;
        }>,
      };
    }
    case 'data-contextUsage':
      return {
        type: 'context.usage',
        usedTokens: Number(data.usedTokens ?? 0),
        maxTokens: Number(data.maxTokens ?? 0),
        percent: Number(data.percent ?? 0),
      };
    case 'data-chatTitle':
      return { type: 'chat.title', title: String(data.title ?? '') };
    case 'data-maxStepsReached':
      return {
        type: 'max_steps',
        maxSteps: Number(data.maxSteps ?? 0),
        canContinue: Boolean(data.canContinue ?? true),
      };
    case 'finish':
      return {
        type: 'message.end',
        finishReason: String(part.finishReason ?? 'stop'),
        usage: part.usage as { promptTokens?: number; completionTokens?: number; totalTokens?: number } | undefined,
      };
    case 'error': {
      const error = (part.error ?? {}) as Record<string, unknown>;
      return {
        type: 'error',
        code: String(error.code ?? 'unknown'),
        message: String(error.message ?? 'Unknown error'),
      };
    }
    default:
      return null;
  }
}

export function encodeSvedaStreamEvent(event: SvedaStreamEvent): string {
  return `data: ${JSON.stringify(event)}\n\n`;
}
