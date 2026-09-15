import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const theme = readFileSync(join(dir, 'theme.css'), 'utf8');
const chat = readFileSync(join(dir, 'components/shell/VedaChat.vue'), 'utf8');
const appearance = readFileSync(join(dir, 'appearance.ts'), 'utf8');
const trigger = readFileSync(join(dir, 'components/shell/VedaMinimizedTrigger.vue'), 'utf8');
const selectContent = readFileSync(join(dir, 'ui/select/SelectContent.vue'), 'utf8');
const tooltipContent = readFileSync(join(dir, 'ui/tooltip/TooltipContent.vue'), 'utf8');
const switchUi = readFileSync(join(dir, 'ui/switch/Switch.vue'), 'utf8');

test('theme.css exposes zero-specificity public veda tokens without cycling host variables', () => {
  assert.match(theme, /:where\(\.veda-chat\)/);
  assert.match(theme, /--veda-background:/);
  assert.match(theme, /--veda-foreground:/);
  assert.match(theme, /--veda-brand:/);
  assert.match(theme, /--veda-radius:\s*0/);
  assert.match(theme, /\.veda-chat-surface/);
  assert.match(theme, /body\.veda-chat-immersive-mode/);
  assert.doesNotMatch(theme, /--background:\s*var\(--veda-background\)/);
  assert.doesNotMatch(theme, /--foreground:\s*var\(--veda-foreground\)/);
  assert.match(chat, /'veda-chat'/);
});

test('portaled select and tooltip keep veda tokens and popover backgrounds', () => {
  assert.match(selectContent, /veda-chat/);
  assert.match(selectContent, /bg-popover/);
  assert.doesNotMatch(selectContent, /bg-input text-popover-foreground/);
  assert.match(tooltipContent, /veda-chat/);
  assert.match(tooltipContent, /bg-popover/);
});

test('appearance helper injects radius and token declarations onto .veda-chat', () => {
  assert.match(appearance, /VEDA_APPEARANCE_STYLE_ID = 'veda-appearance'/);
  assert.match(appearance, /export const applyVedaAppearance/);
  assert.match(appearance, /export const resolveVedaAppearance/);
  assert.match(appearance, /VEDA_APPEARANCE_PRESETS/);
  assert.match(appearance, /preset: 'lms'/);
  assert.match(appearance, /preset: 'ocean'/);
  assert.match(appearance, /preset: 'forest'/);
  assert.match(appearance, /preset: 'sunset'/);
  assert.match(appearance, /preset: 'sand'/);
  assert.match(appearance, /'166 72% 32%'/);
  assert.match(appearance, /\.veda-chat\{/);
  assert.match(appearance, /\.dark \.veda-chat,\.veda-chat\.dark/);
  assert.match(appearance, /sanitizeVedaTheme/);
  assert.match(appearance, /theme === 'dark'/);
  assert.match(appearance, /theme === 'light'/);
  assert.match(appearance, /VEDA_LAUNCHER_ICON_IDS/);
  assert.match(appearance, /useVedaLauncher/);
  assert.match(appearance, /sanitizeVedaLauncher/);
  assert.match(appearance, /sanitizeVedaLauncherImage/);
  assert.match(appearance, /VEDA_LAUNCHER_IMAGE_MAX_BYTES/);
});

test('thinking switch stays a pill with a raised thumb', () => {
  assert.match(switchUi, /rounded-full/);
  assert.match(switchUi, /shadow-inner/);
  assert.match(switchUi, /size === 'sm'/);
  assert.doesNotMatch(switchUi, /rounded-\[var\(--veda-radius\)\]/);
});

test('minimized trigger uses launcher label, icon, and image', () => {
  assert.match(trigger, /vedaLauncherIconComponent/);
  assert.match(chat, /chat.launcherLabel/);
  assert.match(chat, /chat.launcherIcon/);
  assert.match(chat, /chat.launcherImage/);
});
