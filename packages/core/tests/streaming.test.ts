import { describe, expect, it } from 'vitest';
import { iterateVedaStream } from '../src/streaming.js';

function sseResponse(lines: string[]): Response {
  const encoder = new TextEncoder();
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      for (const line of lines) {
        controller.enqueue(encoder.encode(`${line}\n`));
      }
      controller.close();
    },
  });

  return new Response(stream, {
    status: 200,
    headers: { 'Content-Type': 'text/event-stream' },
  });
}

async function collect(response: Response, mode: 'veda' | 'vercel' = 'veda') {
  const events = [];
  for await (const event of iterateVedaStream(response, mode)) {
    events.push(event);
  }
  return events;
}

describe('iterateVedaStream', () => {
  it('parses veda protocol SSE frames', async () => {
    const response = sseResponse([
      'data: {"type":"message.start","chatId":"c1"}',
      '',
      'data: {"type":"text.delta","delta":"Hel"}',
      'data: {"type":"text.delta","delta":"lo"}',
      'data: {"type":"message.end","finishReason":"stop"}',
      'data: [DONE]',
    ]);

    const events = await collect(response);

    expect(events).toHaveLength(4);
    expect(events[1]).toEqual({ type: 'text.delta', delta: 'Hel' });
    expect(events[3]).toEqual({ type: 'message.end', finishReason: 'stop' });
  });

  it('parses vercel data protocol frames in compat mode', async () => {
    const response = sseResponse([
      'data: {"type":"text-delta","textDelta":"Hi"}',
      'data: {"type":"data-maxStepsReached","data":{"maxSteps":30,"canContinue":true}}',
      'data: [DONE]',
    ]);

    const events = await collect(response, 'vercel');

    expect(events).toEqual([
      { type: 'text.delta', delta: 'Hi' },
      { type: 'max_steps', maxSteps: 30, canContinue: true },
    ]);
  });

  it('skips malformed lines without failing the stream', async () => {
    const response = sseResponse([
      'data: {broken',
      ': comment',
      'data: {"type":"text.delta","delta":"ok"}',
    ]);

    const events = await collect(response);
    expect(events).toEqual([{ type: 'text.delta', delta: 'ok' }]);
  });

  it('throws when response has no body', async () => {
    const response = new Response(null, { status: 200 });
    await expect(collect(response)).rejects.toThrow('no body');
  });
});
