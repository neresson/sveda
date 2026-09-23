import { expect, test } from '@playwright/test';
import { signIn } from './helpers/admin';

const deepwikiCatalog = `{
  "mcpServers": {
    "deepwiki": {
      "url": "https://mcp.deepwiki.com/mcp"
    }
  }
}`;

test.describe('admin MCP catalog', () => {
  test('rejects invalid JSON and saves the public DeepWiki server', async ({ page }) => {
    await signIn(page);
    await page.goto('/admin/mcp');
    await expect(page.getByRole('heading', { name: 'MCP.' })).toBeVisible();

    const editor = page.locator('textarea[name="mcp_json"]');
    await expect(editor).toBeVisible();
    await editor.fill('{');
    await page.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText('This is not valid JSON.')).toBeVisible();

    await editor.fill(deepwikiCatalog);
    await page.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText('Saved.')).toBeVisible();

    await page.reload();
    await expect(page.getByRole('heading', { name: 'MCP.' })).toBeVisible();
    await expect(page.locator('textarea[name="mcp_json"]')).toHaveValue(
      /mcp\.deepwiki\.com\/mcp/,
    );
  });

  test('mint rejects a host MCP token without a URL', async ({ request }) => {
    const response = await request.post('/sveda/embed/token', {
      data: {
        visitor_id: 'e2e-mcp-token-only',
        host_mcp_token: 'secret',
      },
    });
    expect(response.status()).toBe(422);
    const payload = (await response.json()) as { message?: string };
    expect(payload.message).toContain('host_mcp_token requires host_mcp_url');
  });
});

test.describe('public DeepWiki MCP', () => {
  test('lists documentation tools over streamable HTTP', async ({ request }) => {
    test.setTimeout(40_000);
    const headers = {
      accept: 'application/json, text/event-stream',
      'content-type': 'application/json',
      'mcp-protocol-version': '2025-11-25',
    };
    const initialize = await request.post('https://mcp.deepwiki.com/mcp', {
      headers,
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'initialize',
        params: {
          protocolVersion: '2025-11-25',
          capabilities: {},
          clientInfo: { name: 'sveda-e2e', version: '0.1.0' },
        },
      },
      timeout: 25_000,
    });
    if (!initialize.ok()) {
      test.skip(true, `DeepWiki MCP unreachable: ${initialize.status()}`);
    }

    const session = initialize.headers()['mcp-session-id'];
    const listed = await request.post('https://mcp.deepwiki.com/mcp', {
      headers: {
        ...headers,
        ...(session ? { 'mcp-session-id': session } : {}),
      },
      data: {
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/list',
        params: {},
      },
      timeout: 25_000,
    });
    if (!listed.ok()) {
      test.skip(true, `DeepWiki tools/list failed: ${listed.status()}`);
    }

    const body = await listed.text();
    expect(body).toContain('read_wiki_structure');
    expect(body).toContain('read_wiki_contents');
    expect(body).toContain('ask_question');
  });
});
