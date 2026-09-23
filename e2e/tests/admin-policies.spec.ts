import { expect, test } from '@playwright/test';
import { ADMIN_KEY, signIn } from './helpers/admin';

const readerPolicies = `{
  "reader": {
    "web": false,
    "code": false,
    "mcp": {
      "allow": ["search_posts"],
      "max_mode": "read"
    },
    "client": {
      "allow": ["ui_*"]
    }
  }
}`;

test.describe('admin embed policies', () => {
  test('saves reader policy JSON and reloads it', async ({ page }) => {
    await signIn(page);
    await page.goto('/admin/policies');
    await expect(page.getByRole('heading', { name: 'Embed policies' })).toBeVisible();

    const editor = page.locator('section textarea');
    await expect(editor).toBeVisible();
    await editor.fill(readerPolicies);
    await page.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText('Saved.')).toBeVisible();

    await page.reload();
    await expect(page.getByRole('heading', { name: 'Embed policies' })).toBeVisible();
    await expect(page.locator('section textarea')).toHaveValue(/"reader"/);
    await expect(page.locator('section textarea')).toHaveValue(/"search_posts"/);
    await expect(page.locator('section textarea')).toHaveValue(/"max_mode": "read"/);
    await expect(page.locator('section textarea')).toHaveValue(/"web": false/);
  });

  test('mint rejects unknown policy and embed config exposes reader capabilities', async ({
    request,
  }) => {
    const save = await request.post('/admin/settings', {
      headers: {
        'content-type': 'application/json',
        'x-sveda-admin-key': ADMIN_KEY,
      },
      data: {
        policies: {
          reader: {
            web: false,
            code: false,
            mcp: { allow: ['search_posts'], max_mode: 'read' },
            client: { allow: ['ui_*'] },
          },
        },
      },
    });
    expect(save.status()).toBe(200);

    const unknown = await request.post('/sveda/embed/token', {
      data: {
        visitor_id: 'e2e-policy-unknown',
        policy: 'missing',
      },
    });
    expect(unknown.status()).toBe(422);
    const unknownBody = (await unknown.json()) as { message?: string };
    expect(unknownBody.message).toBe('unknown policy');

    const mint = await request.post('/sveda/embed/token', {
      data: {
        visitor_id: 'e2e-policy-reader',
        policy: 'reader',
      },
    });
    expect(mint.status()).toBe(200);
    const minted = (await mint.json()) as { token?: string };
    expect(minted.token).toBeTruthy();

    const config = await request.get('/sveda/embed/config', {
      headers: {
        'x-sveda-embed-token': minted.token!,
      },
    });
    expect(config.status()).toBe(200);
    const body = (await config.json()) as {
      capabilities?: {
        restricted?: boolean;
        web?: boolean;
        mcp?: { allow?: string[]; max_mode?: string };
      };
    };
    expect(body.capabilities?.restricted).toBe(true);
    expect(body.capabilities?.web).toBe(false);
    expect(body.capabilities?.mcp?.allow).toEqual(['search_posts']);
    expect(body.capabilities?.mcp?.max_mode).toBe('read');

    const openMint = await request.post('/sveda/embed/token', {
      data: { visitor_id: 'e2e-policy-open' },
    });
    expect(openMint.status()).toBe(200);
    const openToken = ((await openMint.json()) as { token: string }).token;
    const openConfig = await request.get('/sveda/embed/config', {
      headers: { 'x-sveda-embed-token': openToken },
    });
    expect(openConfig.status()).toBe(200);
    const openBody = (await openConfig.json()) as { capabilities?: { restricted?: boolean } };
    expect(openBody.capabilities?.restricted).toBe(false);
  });
});
