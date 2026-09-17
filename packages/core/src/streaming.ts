import {
  parseSvedaStreamLine,
  vercelDataPartToSvedaEvent,
  type SvedaStreamEvent,
} from '@sveda-ai/protocol';

export type SvedaStreamProtocolMode = 'sveda' | 'vercel';

async function* iterateStreamLines(body: ReadableStream<Uint8Array>): AsyncGenerator<string> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }

      buffer += decoder.decode(value, { stream: true });

      let newlineIndex = buffer.indexOf('\n');
      while (newlineIndex >= 0) {
        yield buffer.slice(0, newlineIndex);
        buffer = buffer.slice(newlineIndex + 1);
        newlineIndex = buffer.indexOf('\n');
      }
    }

    const tail = buffer.trim();
    if (tail) {
      yield tail;
    }
  } finally {
    reader.releaseLock();
  }
}

function parseVercelLine(line: string): SvedaStreamEvent | null {
  const trimmed = line.trim();
  if (!trimmed.startsWith('data:')) {
    return null;
  }

  const payload = trimmed.slice(5).trim();
  if (!payload || payload === '[DONE]') {
    return null;
  }

  try {
    const part = JSON.parse(payload) as { type?: string };
    if (!part || typeof part.type !== 'string') {
      return null;
    }

    return vercelDataPartToSvedaEvent(part as { type: string } & Record<string, unknown>);
  } catch {
    return null;
  }
}

export async function* iterateSvedaStream(
  response: Response,
  mode: SvedaStreamProtocolMode = 'sveda'
): AsyncGenerator<SvedaStreamEvent> {
  if (!response.body) {
    throw new Error('[sveda] Stream response has no body');
  }

  for await (const line of iterateStreamLines(response.body)) {
    const event = mode === 'vercel' ? parseVercelLine(line) : parseSvedaStreamLine(line);
    if (event) {
      yield event;
    }
  }
}
