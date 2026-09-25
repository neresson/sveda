import { expect, test } from '@playwright/test';
import {
  enablePublicWebSearch,
  mintEmbedToken,
  openPublicEmbed,
  sendAdminChat,
} from './helpers/chat';
import { hasDeepseekKey } from './helpers/env';
import { parseSse, sseText, sseToolCalls, sseToolResults, type SseEvent } from './helpers/sse';

const live = hasDeepseekKey();

const SEARCH_PROMPT =
  'Search the public web for sveda.dev. You must call the web_search tool. Then reply with the official homepage URL you found. Do not invent the URL.';

function expectLiveWebSearch(events: SseEvent[]) {
  const used = events
    .filter((event) => event.type === 'tool.call')
    .map((event) => event.toolName)
    .filter(Boolean);
  const calls = sseToolCalls(events, 'web_search');
  expect(calls, `expected web_search; tools used: ${used.join(', ') || 'none'}`).not.toHaveLength(0);

  const results = sseToolResults(events, 'web_search');
  expect(results.length).toBeGreaterThan(0);
  const output = results[0]?.output as {
    success?: boolean;
    error?: string;
    data?: { backend?: string; results?: Array<{ url?: string; title?: string }> };
  };
  expect(output?.success, JSON.stringify(output)).toBe(true);
  const hits = output?.data?.results ?? [];
  expect(hits.length, `backend=${output?.data?.backend ?? '?'}`).toBeGreaterThan(0);
  expect(hits.some((hit) => String(hit.url ?? '').startsWith('http'))).toBeTruthy();
  return output;
}

test.describe('live web search from chat', () => {
  test.skip(!live, 'DEEPSEEK_API_KEY is required for live internet search from chat');
  test.setTimeout(180_000);

  test('public embed chat searches the real web', async ({ page, request }) => {
    await enablePublicWebSearch(request);
    const token = await mintEmbedToken(request, `e2e-web-${Date.now()}`);
    await openPublicEmbed(page, token);

    const stream = page.waitForResponse(
      (response) =>
        response.url().includes('/sveda/stream') && response.request().method() === 'POST',
      { timeout: 180_000 },
    );
    await sendAdminChat(page, SEARCH_PROMPT);
    const events = parseSse(await (await stream).text());
    const search = expectLiveWebSearch(events);
    const text = sseText(events);

    await expect(page.getByText(/Search the web|Поиск в интернете/).first()).toBeVisible({
      timeout: 180_000,
    });
    await expect(page.locator('.sveda-chat .prose').first()).toBeVisible({ timeout: 180_000 });
    expect(text.length, text).toBeGreaterThan(0);
    expect(
      /sveda\.dev|https?:\/\//i.test(text),
      `reply should cite a URL after ${search?.data?.backend} search: ${text}`,
    ).toBeTruthy();
  });
});
