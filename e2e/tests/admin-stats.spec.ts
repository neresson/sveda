import { expect, test } from '@playwright/test';
import { openAdmin, signIn } from './helpers/admin';
import { expectDeepseekReply, mintEmbedToken, streamChat } from './helpers/chat';
import { hasDeepseekKey } from './helpers/env';

const live = hasDeepseekKey();

test.describe('admin usage stats', () => {
  test('dashboard and usage pages render request and token cards', async ({ page }) => {
    await signIn(page);
    await expect(page.getByRole('heading', { name: 'Dashboard.' })).toBeVisible();
    await expect(page.getByText('Requests', { exact: true })).toBeVisible();
    await expect(page.getByText('Tokens in', { exact: true })).toBeVisible();
    await expect(page.getByText('Tokens out', { exact: true })).toBeVisible();
    await expect(page.getByText('Tokens total', { exact: true })).toBeVisible();
    await expect(page.getByText('Chats', { exact: true })).toBeVisible();
    await expect(page.getByText('Users', { exact: true })).toBeVisible();

    const empty = page.getByText('No requests in this period yet.');
    if (await empty.isVisible()) {
      await expect(empty).toBeVisible();
      await openAdmin(page, '/admin/usage', 'Usage.');
      await expect(page.getByText('No model usage yet.')).toBeVisible();
      await expect(page.getByText('No requests yet.')).toBeVisible();
      return;
    }

    await expect(page.locator('.text-3xl').first()).not.toHaveText('');
    await openAdmin(page, '/admin/usage', 'Usage.');
    await expect(page.getByRole('columnheader', { name: 'Model' })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: 'Tokens' })).toBeVisible();
  });

  test('a completed chat turn shows up on dashboard and usage', async ({ page, request }) => {
    test.skip(!live, 'DEEPSEEK_API_KEY is required to generate live usage');
    test.setTimeout(180_000);
    const token = await mintEmbedToken(request, 'e2e-stats');
    await expectDeepseekReply(await streamChat(request, token, 'Reply with OK.', false));

    await signIn(page);
    await expect(page.locator('.grid .text-3xl').first()).not.toHaveText('0');

    await openAdmin(page, '/admin/usage', 'Usage.');
    await expect(page.getByText('No requests yet.')).toHaveCount(0);
    await expect(page.getByText(/DeepSeek/i).first()).toBeVisible();
    await expect(page.getByRole('columnheader', { name: 'Tokens' })).toBeVisible();
  });
});
