import { describe, expect, it } from 'vitest';
import { pageFromAdminToolResult } from '../../../apps/runtime/resources/js/admin/adminOperator.js';

describe('pageFromAdminToolResult', () => {
  it('ignores missing parts and non-results', () => {
    expect(pageFromAdminToolResult(null)).toBeNull();
    expect(pageFromAdminToolResult(undefined)).toBeNull();
    expect(
      pageFromAdminToolResult({
        type: 'tool-call',
        toolCallId: 't1',
        toolName: 'admin_open_page',
        output: { page: 'models', reload: true },
      }),
    ).toBeNull();
  });

  it('ignores host tools and incomplete admin results', () => {
    expect(
      pageFromAdminToolResult({
        type: 'tool-result',
        toolCallId: 't1',
        toolName: 'spawn_tasks',
        output: { success: true, page: 'dashboard' },
      }),
    ).toBeNull();
    expect(
      pageFromAdminToolResult({
        type: 'tool-result',
        toolName: 'admin_dashboard',
        output: { success: true, page: 'dashboard' },
      }),
    ).toBeNull();
    expect(
      pageFromAdminToolResult({
        type: 'tool-result',
        toolCallId: '',
        toolName: 'admin_dashboard',
        output: { success: true, page: 'dashboard' },
      }),
    ).toBeNull();
  });

  it('ignores failed operator tools so the console stays put', () => {
    expect(
      pageFromAdminToolResult({
        type: 'tool-result',
        toolCallId: 't1',
        toolName: 'admin_update_settings',
        output: { success: false, error: 'Could not save admin settings.' },
      }),
    ).toBeNull();
  });

  it('syncs the console after a successful read that names a page', () => {
    expect(
      pageFromAdminToolResult({
        type: 'tool-result',
        toolCallId: 'dash-1',
        toolName: 'admin_dashboard',
        output: { success: true, page: 'dashboard', reload: false, data: { requests: 0 } },
      }),
    ).toEqual({
      id: 'dash-1',
      page: 'dashboard',
      url: '',
      reload: true,
    });
  });

  it('syncs writes and open_page, including urls from tool data', () => {
    expect(
      pageFromAdminToolResult({
        type: 'tool-result',
        toolCallId: 'upd-1',
        toolName: 'admin_update_settings',
        output: {
          success: true,
          page: 'mcp',
          reload: true,
          data: { mcp: { mcpServers: {} } },
        },
      }),
    ).toEqual({
      id: 'upd-1',
      page: 'mcp',
      url: '',
      reload: true,
    });

    expect(
      pageFromAdminToolResult({
        type: 'tool-result',
        toolCallId: 'open-1',
        toolName: 'admin_open_page',
        output: {
          page: '  models  ',
          data: { url: ' /admin/models ' },
        },
      }),
    ).toEqual({
      id: 'open-1',
      page: 'models',
      url: '/admin/models',
      reload: true,
    });
  });

  it('returns null when there is nothing to sync', () => {
    expect(
      pageFromAdminToolResult({
        type: 'tool-result',
        toolCallId: 't1',
        toolName: 'admin_get_settings',
        output: { success: true, reload: false, data: {} },
      }),
    ).toBeNull();
    expect(
      pageFromAdminToolResult({
        type: 'tool-result',
        toolCallId: 't2',
        toolName: 'admin_usage',
        output: 'not-an-object',
      }),
    ).toBeNull();
  });
});
