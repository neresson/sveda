import { expect, test } from '@playwright/test';
import {
  expectDeepseekReply,
  jsonMessage,
  mintEmbedToken,
  streamChat,
} from './helpers/chat';
import { hasDeepseekKey } from './helpers/env';

const live = hasDeepseekKey();

test.describe('chat protocol', () => {
  test('health is up and stream without a token is unauthorized', async ({ request }) => {
    const health = await request.get('/sveda/health');
    expect(health.ok()).toBeTruthy();
    const payload = (await health.json()) as { ok?: boolean; runtime?: string };
    expect(payload.ok).toBe(true);
    expect(payload.runtime).toBe('rust');

    const stream = await request.post('/sveda/stream', {
      headers: { accept: 'text/event-stream', 'content-type': 'application/json' },
      data: { messages: [{ id: 'm1', role: 'user', content: 'hi' }] },
    });
    expect(stream.status()).toBe(401);

    const message = await request.post('/sveda/message', {
      data: { prompt: 'hi' },
    });
    expect(message.status()).toBe(401);
  });
});

test.describe('live chat', () => {
  test.skip(!live, 'DEEPSEEK_API_KEY is required for live DeepSeek tests');
  test.setTimeout(180_000);

  test('reaches DeepSeek with thinking off and on', async ({ request }) => {
    const token = await mintEmbedToken(request, 'e2e-deepseek');

    const plain = await expectDeepseekReply(
      await streamChat(request, token, 'Reply with exactly the word PONG and nothing else.', false),
    );
    expect(plain.text.toUpperCase()).toContain('PONG');

    const thinking = await expectDeepseekReply(
      await streamChat(request, token, 'What is 8 + 5? Reply with the number only.', true),
    );
    expect(thinking.text).toMatch(/13/);

    const json = await jsonMessage(
      request,
      token,
      'Reply with exactly the word PONG and nothing else.',
    );
    expect(json.explanation.toUpperCase()).toContain('PONG');
    expect(json.tokens_used).toBeGreaterThan(0);
  });
});
