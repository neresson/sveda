import { describe, expect, it } from 'vitest';
import {
  VEDA_PROTOCOL_VERSION,
  VEDA_STREAM_EVENTS,
  encodeVedaStreamEvent,
  isVedaStreamEvent,
  parseVedaStreamLine,
  vedaEventToVercelDataPart,
  vercelDataPartToVedaEvent,
  type VedaStreamEvent,
} from '../src/index.js';
import streamEventSchema from '../schemas/stream-event.schema.json' with { type: 'json' };

const sampleEvents: VedaStreamEvent[] = [
  { type: 'message.start', chatId: 'c1', messageId: 'm1' },
  { type: 'text.delta', delta: 'Hello' },
  { type: 'reasoning.delta', delta: 'thinking…' },
  {
    type: 'tool.call',
    toolCallId: 'tc1',
    toolName: 'get_courses',
    target: 'backend',
    input: { limit: 5 },
  },
  {
    type: 'tool.call',
    toolCallId: 'tc2',
    toolName: 'confirm_course_creation',
    target: 'frontend',
    input: { title: 'New course' },
  },
  {
    type: 'tool.result',
    toolCallId: 'tc1',
    toolName: 'get_courses',
    output: { courses: [] },
    renderHint: 'resource_links',
    renderData: { links: [] },
  },
  {
    type: 'tool.progress',
    phase: 'searching',
    tasks: [{ id: 't1', label: 'Search KB', status: 'running' }],
  },
  { type: 'context.usage', usedTokens: 100, maxTokens: 128000, percent: 0.1 },
  { type: 'chat.title', title: 'My chat' },
  { type: 'max_steps', maxSteps: 30, canContinue: true },
  { type: 'message.end', finishReason: 'stop', usage: { totalTokens: 42 } },
  { type: 'error', code: 'rate_limited', message: 'Slow down' },
];

describe('protocol constants', () => {
  it('declares version 1.0', () => {
    expect(VEDA_PROTOCOL_VERSION).toBe('1.0');
  });

  it('covers every event type in the JSON schema', () => {
    const schemaConsts = JSON.stringify(streamEventSchema);
    for (const eventType of VEDA_STREAM_EVENTS) {
      expect(schemaConsts).toContain(`"const":"${eventType}"`);
    }
  });
});

describe('SSE encode/parse round-trip', () => {
  it.each(sampleEvents)('round-trips %s', event => {
    const encoded = encodeVedaStreamEvent(event);
    expect(encoded.startsWith('data: ')).toBe(true);
    expect(encoded.endsWith('\n\n')).toBe(true);

    const parsed = parseVedaStreamLine(encoded);
    expect(parsed).toEqual(event);
  });

  it('ignores non-data lines and [DONE]', () => {
    expect(parseVedaStreamLine('event: message')).toBeNull();
    expect(parseVedaStreamLine('data: [DONE]')).toBeNull();
    expect(parseVedaStreamLine('')).toBeNull();
    expect(parseVedaStreamLine('data: {invalid json')).toBeNull();
  });

  it('isVedaStreamEvent rejects unknown types', () => {
    expect(isVedaStreamEvent({ type: 'text.delta', delta: 'x' })).toBe(true);
    expect(isVedaStreamEvent({ type: 'nope' })).toBe(false);
    expect(isVedaStreamEvent(null)).toBe(false);
    expect(isVedaStreamEvent('text.delta')).toBe(false);
  });
});

describe('Vercel AI SDK adapter', () => {
  it('maps text.delta to text-delta data part', () => {
    expect(vedaEventToVercelDataPart({ type: 'text.delta', delta: 'hi' })).toEqual({
      type: 'text-delta',
      textDelta: 'hi',
    });
  });

  it('maps max_steps to data-maxStepsReached (current ScorpioLMS contract)', () => {
    expect(
      vedaEventToVercelDataPart({ type: 'max_steps', maxSteps: 30, canContinue: true })
    ).toEqual({
      type: 'data-maxStepsReached',
      data: { maxSteps: 30, canContinue: true },
    });
  });

  it('maps context.usage to data-contextUsage', () => {
    const part = vedaEventToVercelDataPart({
      type: 'context.usage',
      usedTokens: 10,
      maxTokens: 100,
      percent: 10,
    });
    expect(part?.type).toBe('data-contextUsage');
  });

  it('maps chat.title to data-chatTitle', () => {
    expect(vedaEventToVercelDataPart({ type: 'chat.title', title: 'T' })?.type).toBe(
      'data-chatTitle'
    );
  });

  it('maps tool.progress to data-toolProgress', () => {
    expect(
      vedaEventToVercelDataPart({
        type: 'tool.progress',
        tasks: [{ id: 'a', label: 'A', status: 'pending' }],
      })?.type
    ).toBe('data-toolProgress');
  });

  it('message.start maps to null (no Vercel equivalent)', () => {
    expect(vedaEventToVercelDataPart({ type: 'message.start' })).toBeNull();
  });

  it('round-trips vercel data parts back to veda events', () => {
    const back = vercelDataPartToVedaEvent({ type: 'text-delta', textDelta: 'yo' });
    expect(back).toEqual({ type: 'text.delta', delta: 'yo' });

    const maxSteps = vercelDataPartToVedaEvent({
      type: 'data-maxStepsReached',
      data: { maxSteps: 50, canContinue: false },
    });
    expect(maxSteps).toEqual({ type: 'max_steps', maxSteps: 50, canContinue: false });
  });

  it('vercelDataPartToVedaEvent returns null for unknown parts', () => {
    expect(vercelDataPartToVedaEvent({ type: 'source' })).toBeNull();
  });
});
