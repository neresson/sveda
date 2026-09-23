import { expect, test } from '@playwright/test';
import { signIn } from './helpers/admin';
import { openAdminChat, sendAdminChat } from './helpers/chat';

const ADMIN_TOOLS = [
  'admin_dashboard',
  'admin_usage',
  'admin_get_settings',
  'admin_update_settings',
  'admin_upsert_model',
  'admin_remove_model',
  'admin_open_page',
];

type StreamBody = {
  context?: { admin?: { page?: string } };
  clientTools?: Array<{ name?: string }>;
};

function sse(events: unknown[]) {
  return `${events.map((event) => `data: ${JSON.stringify(event)}`).join('\n\n')}\n\ndata: [DONE]\n`;
}

function operatorStream(toolName: string, input: unknown, output: unknown, text: string) {
  return sse([
    { type: 'message.start' },
    {
      type: 'tool.call',
      toolCallId: 'op-1',
      toolName,
      target: 'backend',
      input,
    },
    {
      type: 'tool.result',
      toolCallId: 'op-1',
      toolName,
      output,
    },
    { type: 'text.delta', delta: text },
    { type: 'message.end', finishReason: 'stop' },
  ]);
}

test.describe('admin chat operator', () => {
  test('embed mint rejects the reserved admin visitor', async ({ request }) => {
    for (const visitor_id of ['sveda-admin', 'SVEDA-ADMIN', 'Sveda-Admin']) {
      const response = await request.post('/sveda/embed/token', {
        data: { visitor_id },
      });
      expect(response.status(), visitor_id).toBe(422);
      const payload = (await response.json()) as { message?: string };
      expect(payload.message).toContain('reserved');
    }
  });

  test('sends the current admin page and keeps operator tools off the client', async ({
    page,
  }) => {
    const captured: StreamBody[] = [];
    await page.route('**/sveda/stream', async (route) => {
      if (route.request().method() === 'POST') {
        captured.push((route.request().postDataJSON() ?? {}) as StreamBody);
      }
      await route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body: sse([
          { type: 'message.start' },
          { type: 'text.delta', delta: 'On the MCP page.' },
          { type: 'message.end', finishReason: 'stop' },
        ]),
      });
    });

    await signIn(page);
    await page.goto('/admin/mcp');
    await expect(page.getByRole('heading', { name: 'MCP.' })).toBeVisible();
    await openAdminChat(page);
    await sendAdminChat(page, 'Where am I?');
    await expect(page.getByText('On the MCP page.')).toBeVisible();

    expect(captured.length).toBeGreaterThan(0);
    const last = captured.at(-1);
    expect(last?.context?.admin?.page).toBe('mcp');
    const clientTools = last?.clientTools ?? [];
    expect(clientTools.some((tool) => ADMIN_TOOLS.includes(String(tool.name ?? '')))).toBe(
      false,
    );
  });

  test('opens an admin page after admin_open_page', async ({ page }) => {
    await page.route('**/sveda/stream', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body: operatorStream(
          'admin_open_page',
          { page: 'models' },
          {
            success: true,
            page: 'models',
            reload: true,
            data: { page: 'models', url: '/admin/models' },
          },
          'Opened models.',
        ),
      });
    });

    await signIn(page);
    await openAdminChat(page);
    await sendAdminChat(page, 'Open models');
    await expect(page.getByText('Opened models.')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Models.' })).toBeVisible();
    await expect(page).toHaveURL(/\/admin\/models$/);
  });

  test('reloads the matching settings page after a successful write', async ({ page }) => {
    await page.route('**/sveda/stream', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body: operatorStream(
          'admin_update_settings',
          { welcome_message: 'Hello from chat' },
          {
            success: true,
            page: 'prompts',
            reload: true,
            data: { welcome_message: 'Hello from chat', system_prompt: '' },
          },
          'Prompts saved.',
        ),
      });
    });

    await signIn(page);
    await openAdminChat(page);
    await sendAdminChat(page, 'Update the welcome message');
    await expect(page.getByText('Prompts saved.')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Prompts.' })).toBeVisible();
    await expect(page).toHaveURL(/\/admin\/prompts$/);
  });

  test('does not navigate when an operator tool fails', async ({ page }) => {
    await page.route('**/sveda/stream', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body: operatorStream(
          'admin_update_settings',
          {},
          { success: false, error: 'Provide at least one settings field to change.' },
          'I need a patch.',
        ),
      });
    });

    await signIn(page);
    await expect(page.getByRole('heading', { name: 'Dashboard.' })).toBeVisible();
    await openAdminChat(page);
    await sendAdminChat(page, 'Save nothing');
    await expect(page.getByText('I need a patch.')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Dashboard.' })).toBeVisible();
    await expect(page).toHaveURL(/\/admin\/?$/);
  });
});
