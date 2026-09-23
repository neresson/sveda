export type SseEvent = {
  type?: string;
  delta?: string;
  finishReason?: string;
  explanation?: string;
  message?: string;
  code?: string;
  toolName?: string;
  toolCallId?: string;
  input?: Record<string, unknown>;
  output?: unknown;
};

export function parseSse(body: string): SseEvent[] {
  const events: SseEvent[] = [];
  for (const line of body.split('\n')) {
    const payload = line.startsWith('data:') ? line.slice(5).trim() : '';
    if (!payload || payload === '[DONE]') {
      continue;
    }
    try {
      events.push(JSON.parse(payload) as SseEvent);
    } catch {
      // ignore keep-alives
    }
  }
  return events;
}

export function sseText(events: SseEvent[]): string {
  return events
    .filter((event) => event.type === 'text.delta')
    .map((event) => event.delta ?? '')
    .join('');
}

export function sseReasoning(events: SseEvent[]): string {
  return events
    .filter((event) => event.type === 'reasoning.delta')
    .map((event) => event.delta ?? '')
    .join('');
}

export function sseToolCalls(events: SseEvent[], name: string): SseEvent[] {
  return events.filter((event) => event.type === 'tool.call' && event.toolName === name);
}

export function sseToolResults(events: SseEvent[], name: string): SseEvent[] {
  return events.filter((event) => event.type === 'tool.result' && event.toolName === name);
}
