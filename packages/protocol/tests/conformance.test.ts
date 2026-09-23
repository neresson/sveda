import { describe, expect, it } from 'vitest';
import {
  SVEDA_PROTOCOL_VERSION,
  SVEDA_STREAM_EVENTS,
  encodeSvedaStreamEvent,
  isSvedaStreamEvent,
  parseSvedaStreamLine,
  svedaEventToVercelDataPart,
  vercelDataPartToSvedaEvent,
  type SvedaStreamEvent,
} from '../src/index.js';
import streamEventSchema from '../schemas/stream-event.schema.json' with { type: 'json' };

const sampleEvents: SvedaStreamEvent[] = [
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
    confirmation: 'required',
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
    expect(SVEDA_PROTOCOL_VERSION).toBe('1.0');
  });

  it('covers every event type in the JSON schema', () => {
    const schemaConsts = JSON.stringify(streamEventSchema);
    for (const eventType of SVEDA_STREAM_EVENTS) {
      expect(schemaConsts).toContain(`"const":"${eventType}"`);
    }
  });
});

describe('SSE encode/parse round-trip', () => {
  it.each(sampleEvents)('round-trips %s', event => {
    const encoded = encodeSvedaStreamEvent(event);
    expect(encoded.startsWith('data: ')).toBe(true);
    expect(encoded.endsWith('\n\n')).toBe(true);

    const parsed = parseSvedaStreamLine(encoded);
    expect(parsed).toEqual(event);
  });

  it('ignores non-data lines and [DONE]', () => {
    expect(parseSvedaStreamLine('event: message')).toBeNull();
    expect(parseSvedaStreamLine('data: [DONE]')).toBeNull();
    expect(parseSvedaStreamLine('')).toBeNull();
    expect(parseSvedaStreamLine('data: {invalid json')).toBeNull();
  });

  it('isSvedaStreamEvent rejects unknown types', () => {
    expect(isSvedaStreamEvent({ type: 'text.delta', delta: 'x' })).toBe(true);
    expect(isSvedaStreamEvent({ type: 'nope' })).toBe(false);
    expect(isSvedaStreamEvent(null)).toBe(false);
    expect(isSvedaStreamEvent('text.delta')).toBe(false);
  });
});

describe('Vercel AI SDK adapter', () => {
  it('maps text.delta to text-delta data part', () => {
    expect(svedaEventToVercelDataPart({ type: 'text.delta', delta: 'hi' })).toEqual({
      type: 'text-delta',
      textDelta: 'hi',
    });
  });

  it('maps max_steps to data-maxStepsReached (current ScorpioLMS contract)', () => {
    expect(
      svedaEventToVercelDataPart({ type: 'max_steps', maxSteps: 30, canContinue: true })
    ).toEqual({
      type: 'data-maxStepsReached',
      data: { maxSteps: 30, canContinue: true },
    });
  });

  it('maps context.usage to data-contextUsage', () => {
    const part = svedaEventToVercelDataPart({
      type: 'context.usage',
      usedTokens: 10,
      maxTokens: 100,
      percent: 10,
    });
    expect(part?.type).toBe('data-contextUsage');
  });

  it('maps chat.title to data-chatTitle', () => {
    expect(svedaEventToVercelDataPart({ type: 'chat.title', title: 'T' })?.type).toBe(
      'data-chatTitle'
    );
  });

  it('maps tool.progress to data-toolProgress', () => {
    expect(
      svedaEventToVercelDataPart({
        type: 'tool.progress',
        tasks: [{ id: 'a', label: 'A', status: 'pending' }],
      })?.type
    ).toBe('data-toolProgress');
  });

  it('message.start maps to null (no Vercel equivalent)', () => {
    expect(svedaEventToVercelDataPart({ type: 'message.start' })).toBeNull();
  });

  it('round-trips vercel data parts back to sveda events', () => {
    const back = vercelDataPartToSvedaEvent({ type: 'text-delta', textDelta: 'yo' });
    expect(back).toEqual({ type: 'text.delta', delta: 'yo' });

    const maxSteps = vercelDataPartToSvedaEvent({
      type: 'data-maxStepsReached',
      data: { maxSteps: 50, canContinue: false },
    });
    expect(maxSteps).toEqual({ type: 'max_steps', maxSteps: 50, canContinue: false });
  });

  it('vercelDataPartToSvedaEvent returns null for unknown parts', () => {
    expect(vercelDataPartToSvedaEvent({ type: 'source' })).toBeNull();
  });
});
