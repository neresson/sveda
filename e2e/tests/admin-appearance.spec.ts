import { expect, test } from '@playwright/test';
import { openAdmin, signIn } from './helpers/admin';

test.describe('admin shell', () => {
  test('signs in and keeps working sections reachable', async ({ page }) => {
    await signIn(page);
    await openAdmin(page, '/admin/appearance', 'Appearance.');
    await openAdmin(page, '/admin/models', 'Models.');
    await expect(page.getByRole('heading', { name: 'Embeddings.' })).toBeVisible();
    await openAdmin(page, '/admin/prompts', 'Prompts.');
    await openAdmin(page, '/admin/runtime', 'Runtime.');
    await expect(page.locator('input[name="web_enabled"]')).toBeChecked();
    await openAdmin(page, '/admin/security', 'Security.');
    await openAdmin(page, '/admin/mcp', 'MCP.');
    await openAdmin(page, '/admin/usage', 'Usage.');
    await openAdmin(page, '/admin', 'Dashboard.');
  });

  test('shows the admin chat widget after login', async ({ page }) => {
    await signIn(page);
    const launcher = page.locator('.sveda-chat.fixed button').first();
    await expect(launcher).toBeVisible();
    await launcher.click();
    await expect(page.locator('.sveda-chat-frame').first()).toBeVisible();
  });
});

test.describe('admin appearance', () => {
  test('shows distinct presets and links to appearance docs', async ({ page }) => {
    await signIn(page);
    await page.goto('/admin/appearance');
    await expect(page.getByRole('heading', { name: 'Appearance.' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Appearance docs' })).toHaveAttribute(
      'href',
      'https://sveda.dev/docs/appearance',
    );
    await expect(page.getByText('Ink', { exact: true })).toBeVisible();
    await expect(page.getByText('Ocean', { exact: true })).toBeVisible();
    await expect(page.getByText('Default', { exact: true })).toBeVisible();

    const ink = page.locator('button').filter({ hasText: /^Ink/ }).first();
    const lms = page.locator('button').filter({ hasText: /^LMS/ }).first();
    const inkBrand = ink.locator('span.h-7').first();
    const lmsBrand = lms.locator('span.h-8, span.h-7').first();
    await expect(inkBrand).toHaveCSS('background-color', 'rgb(10, 10, 10)');
    await expect(lmsBrand).not.toHaveCSS('background-color', 'rgb(10, 10, 10)');
  });
});

test.describe('embed default look', () => {
  test('iframe shell keeps the ink dashed frame when appearance is empty', async ({
    request,
    page,
  }) => {
    const mint = await request.post('/sveda/embed/token', {
      data: { visitor_id: 'e2e-embed' },
    });
    expect(mint.ok()).toBeTruthy();
    const { token } = (await mint.json()) as { token: string };
    expect(token).toMatch(/^sveda_embed_/);

    const config = await request.get('/sveda/embed/config', {
      headers: { 'x-sveda-embed-token': token },
    });
    expect(config.ok()).toBeTruthy();
    const payload = (await config.json()) as { appearance: Record<string, unknown> };
    expect(payload.appearance).toEqual({});

    await page.goto(`/sveda/embed?token=${encodeURIComponent(token)}`);
    const host = page.locator('#sveda-embed .sveda-chat-host');
    await expect(page.locator('#sveda-embed')).toBeVisible();
    await expect(host).toBeVisible();
    await expect(page.locator('#sveda-embed .sveda-chat-frame').first()).toBeVisible();
    const brand = await host.evaluate((node) => {
      return getComputedStyle(node).getPropertyValue('--sveda-brand').trim();
    });
    expect(brand).toBe('0 0% 4%');
    const radius = await host.evaluate((node) => {
      return getComputedStyle(node).getPropertyValue('--sveda-radius').trim();
    });
    expect(radius === '0' || radius === '0px').toBeTruthy();
  });
});
