import { expect, test } from '@playwright/test';
import { signIn } from './helpers/admin';
import {
  expectDeepseekReply,
  jsonMessage,
  mintEmbedToken,
  openAdminChat,
  sendAdminChat,
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

test.describe('admin chat widget', () => {
  test('posts thinking false when the reasoning switch is off', async ({ page }) => {
    let captured: { options?: { thinking?: boolean }; prompt?: string } | null = null;
    await page.route('**/sveda/stream', async (route) => {
      captured = route.request().postDataJSON() as typeof captured;
      await route.fulfill({
        status: 200,
        headers: { 'content-type': 'text/event-stream' },
        body: [
          'data: {"type":"message.start"}',
          'data: {"type":"text.delta","delta":"MOCK-REPLY"}',
          'data: {"type":"message.end","finishReason":"stop"}',
          '',
        ].join('\n\n'),
      });
    });

    await signIn(page);
    await openAdminChat(page);
    const thinkingSwitch = page.getByRole('switch', { name: /Chain-of-thought/i });
    await expect(thinkingSwitch).toBeVisible();
    await expect(thinkingSwitch).toHaveAttribute('data-state', 'checked');
    await thinkingSwitch.click();
    await expect(thinkingSwitch).toHaveAttribute('data-state', 'unchecked');
    await sendAdminChat(page, 'hello from e2e');
    await expect(page.getByText('MOCK-REPLY').first()).toBeVisible();
    expect(captured?.prompt).toBe('hello from e2e');
    expect(captured?.options?.thinking).toBe(false);
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

  test('admin widget sends a prompt and shows the assistant reply', async ({ page }) => {
    await signIn(page);
    await openAdminChat(page);
    await sendAdminChat(page, 'Reply with exactly the word PONG and nothing else.');
    await expect(page.locator('.sveda-chat .prose').getByText(/PONG/i)).toBeVisible({
      timeout: 120_000,
    });
  });
});
