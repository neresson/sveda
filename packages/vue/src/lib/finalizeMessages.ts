export type SvedaFinalizableMessage = {
  parts?: Array<{
    type?: string;
    state?: string;
    output?: unknown;
    result?: unknown;
    [key: string]: unknown;
  }>;
  streaming?: boolean;
  activity?: {
    phase?: string;
    steps?: Array<{ status?: string; [key: string]: unknown }>;
    [key: string]: unknown;
  };
  [key: string]: unknown;
};

const TERMINAL_ACTIVITY_PHASES = new Set(['done', 'error', 'stopped']);

const TOOL_COMPLETE_STATES = new Set([
  'result',
  'output-available',
  'output-error',
  'output-denied',
  'done',
]);

const isToolPart = (part: { type?: string } | undefined): boolean =>
  part?.type === 'dynamic-tool' ||
  (typeof part?.type === 'string' && part.type.startsWith('tool-'));

function finalizeActivity<T extends SvedaFinalizableMessage>(message: T): T {
  if (!message.activity) {
    return message;
  }

  const activity = { ...message.activity };
  if (activity.phase && !TERMINAL_ACTIVITY_PHASES.has(activity.phase)) {
    activity.phase = 'done';
  }

  if (Array.isArray(activity.steps)) {
    activity.steps = activity.steps.map(step => ({
      ...step,
      status: step.status === 'running' ? 'success' : step.status,
    }));
  }

  return { ...message, activity };
}

function finalizeParts<T extends SvedaFinalizableMessage>(message: T): T {
  if (!Array.isArray(message.parts)) {
    if (message.streaming === true) {
      return { ...message, streaming: false };
    }
    return message;
  }

  const parts = message.parts.map(part => {
    if (part?.state === 'streaming') {
      return { ...part, state: 'done' };
    }
    if (isToolPart(part) && part.state && !TOOL_COMPLETE_STATES.has(part.state)) {
      if (part.output !== undefined || part.result !== undefined) {
        return { ...part, state: 'output-available' };
      }
    }
    return part;
  });

  return {
    ...message,
    parts,
    streaming: false,
  };
}

export function finalizeMessagesForDisplay<T extends SvedaFinalizableMessage>(messages: T[]): T[] {
  return messages.map(message => finalizeActivity(finalizeParts(message)));
}
