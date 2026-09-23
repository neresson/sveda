import { expect, type APIRequestContext, type Page } from '@playwright/test';
import { ADMIN_KEY } from './admin';
import { parseSse, sseReasoning, sseText, type SseEvent } from './sse';

export async function mintEmbedToken(request: APIRequestContext, visitorId: string) {
  const mint = await request.post('/sveda/embed/token', {
    data: { visitor_id: visitorId },
  });
  expect(mint.ok()).toBeTruthy();
  const { token } = (await mint.json()) as { token: string };
  expect(token).toMatch(/^sveda_embed_/);
  return token;
}

export async function streamChat(
  request: APIRequestContext,
  token: string,
  prompt: string,
  thinking: boolean,
): Promise<SseEvent[]> {
  const response = await request.post('/sveda/stream', {
    headers: {
      accept: 'text/event-stream',
      'content-type': 'application/json',
      'x-sveda-embed-token': token,
    },
    data: {
      prompt,
      messages: [{ id: 'm1', role: 'user', content: prompt }],
      chatId: `e2e-${thinking ? 'think' : 'plain'}-${Date.now()}`,
      options: { thinking },
    },
    timeout: 120_000,
  });
  const body = await response.text();
  expect(response.ok(), body).toBeTruthy();
  return parseSse(body);
}

export async function jsonMessage(
  request: APIRequestContext,
  token: string,
  prompt: string,
  thinking = false,
) {
  const response = await request.post('/sveda/message', {
    headers: {
      'content-type': 'application/json',
      'x-sveda-embed-token': token,
    },
    data: {
      prompt,
      chatId: `e2e-json-${Date.now()}`,
      options: { thinking },
    },
    timeout: 120_000,
  });
  const body = await response.text();
  expect(response.ok(), body).toBeTruthy();
  return JSON.parse(body) as { explanation: string; tokens_used: number; chat_id: string };
}

export async function expectDeepseekReply(events: SseEvent[]) {
  const types = events.map((event) => event.type);
  const errors = events
    .filter((event) => event.type === 'error')
    .map((event) => event.message ?? event.code ?? 'error');
  expect(types, `stream errors: ${errors.join('; ')}`).toContain('message.start');
  expect(types).toContain('text.delta');
  expect(types).toContain('message.end');
  const text = sseText(events).trim();
  expect(text.length).toBeGreaterThan(0);
  return { text, reasoning: sseReasoning(events), types };
}

export async function openAdminChat(page: Page) {
  const launcher = page.locator('.sveda-chat.fixed button').first();
  await expect(launcher).toBeVisible();
  await launcher.click();
  await expect(page.locator('.sveda-chat-input-textarea')).toBeVisible();
}

export async function sendAdminChat(page: Page, prompt: string) {
  const input = page.locator('.sveda-chat-input-textarea');
  await input.fill(prompt);
  await input.press('Enter');
}

export async function enablePublicWebSearch(request: APIRequestContext) {
  const response = await request.post('/admin/settings', {
    headers: {
      accept: 'application/json',
      'content-type': 'application/json',
      'x-sveda-admin-key': ADMIN_KEY,
    },
    data: { web: { enabled: true } },
  });
  expect(response.ok(), await response.text()).toBeTruthy();
}

export async function openPublicEmbed(page: Page, token: string) {
  await page.addInitScript(() => {
    window.localStorage.setItem('locale', 'en');
  });
  await page.goto(`/sveda/embed?token=${encodeURIComponent(token)}`);
  await expect(page.locator('.sveda-chat-input-textarea')).toBeVisible();
}

export { ADMIN_KEY };
