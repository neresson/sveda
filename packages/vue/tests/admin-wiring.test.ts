import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const root = join(dirname(fileURLToPath(import.meta.url)), '../../..');
const appearanceApp = readFileSync(
  join(root, 'apps/runtime/resources/js/admin/AppearanceApp.vue'),
  'utf8',
);
const createAdmin = readFileSync(
  join(root, 'apps/runtime/resources/js/admin/createAdminSveda.js'),
  'utf8',
);
const adminApp = readFileSync(join(root, 'apps/runtime/resources/js/admin/app.js'), 'utf8');
const adminShell = readFileSync(
  join(root, 'apps/runtime/resources/js/admin/AdminApp.vue'),
  'utf8',
);
const adminChat = readFileSync(
  join(root, 'apps/runtime/resources/js/admin/AdminSvedaChat.vue'),
  'utf8',
);
const adminOperator = readFileSync(
  join(root, 'apps/runtime/resources/js/admin/adminOperator.js'),
  'utf8',
);

describe('admin appearance wiring', () => {
  it('builds swatches from package presets, not an empty server map', () => {
    expect(appearanceApp).toContain('SVEDA_APPEARANCE_PRESETS');
    expect(appearanceApp).toContain('https://sveda.dev/docs/appearance');
    expect(appearanceApp).toContain('appearance.host_priority');
  });

  it('mints an admin chat session and passes saved appearance into createSveda', () => {
    expect(adminApp).toContain('payload.chat?.sessionUrl');
    expect(adminApp).toContain('payload.settings?.appearance');
    expect(createAdmin).toContain('window.location.origin');
    expect(createAdmin).toContain('appearance: appearance && typeof appearance === \'object\' ? appearance : {}');
  });

  it('maps catalog thinking flags onto admin chat models', () => {
    expect(createAdmin).toContain('supportsThinking: Boolean(model?.supportsThinking ?? model?.supports_thinking ?? model?.thinking)');
  });

  it('reloads the admin console after operator tool results', () => {
    expect(adminShell).toContain("provide('reloadAdmin'");
    expect(adminChat).toContain('pageFromAdminToolResult');
    expect(adminChat).toContain("client.contextRegistry.register('admin'");
    expect(adminOperator).toContain("toolName.startsWith(ADMIN_TOOL_PREFIX)");
  });

  it('keeps the operator catalog on the server, not as client tools', () => {
    const adminTools = readFileSync(
      join(root, 'crates/sveda-server/src/admin_tools.rs'),
      'utf8',
    );
    for (const name of [
      'admin_dashboard',
      'admin_usage',
      'admin_get_settings',
      'admin_update_settings',
      'admin_upsert_model',
      'admin_remove_model',
      'admin_open_page',
    ]) {
      expect(adminTools).toContain(`"${name}"`);
    }
    expect(createAdmin).not.toContain('admin_dashboard');
    expect(adminChat).not.toContain('clientTools');
  });
});
