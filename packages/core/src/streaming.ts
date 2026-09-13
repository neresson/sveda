import {
  parseVedaStreamLine,
  vercelDataPartToVedaEvent,
  type VedaStreamEvent,
} from '@veda-ai/protocol';

export type VedaStreamProtocolMode = 'veda' | 'vercel';

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

function parseVercelLine(line: string): VedaStreamEvent | null {
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

    return vercelDataPartToVedaEvent(part as { type: string } & Record<string, unknown>);
  } catch {
    return null;
  }
}

export async function* iterateVedaStream(
  response: Response,
  mode: VedaStreamProtocolMode = 'veda'
): AsyncGenerator<VedaStreamEvent> {
  if (!response.body) {
    throw new Error('[veda] Stream response has no body');
  }

  for await (const line of iterateStreamLines(response.body)) {
    const event = mode === 'vercel' ? parseVercelLine(line) : parseVedaStreamLine(line);
    if (event) {
      yield event;
    }
  }
}
