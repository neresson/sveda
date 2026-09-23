import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const theme = readFileSync(join(dir, 'theme.css'), 'utf8');
const chat = readFileSync(join(dir, 'components/shell/SvedaChat.vue'), 'utf8');
const appearanceVue = readFileSync(join(dir, 'appearance.ts'), 'utf8');
const appearance = readFileSync(join(dir, '../../chat/src/appearance.ts'), 'utf8');
const trigger = readFileSync(join(dir, 'components/shell/SvedaMinimizedTrigger.vue'), 'utf8');
const selectContent = readFileSync(join(dir, 'ui/select/SelectContent.vue'), 'utf8');
const tooltipContent = readFileSync(join(dir, 'ui/tooltip/TooltipContent.vue'), 'utf8');
const switchUi = readFileSync(join(dir, 'ui/switch/Switch.vue'), 'utf8');

test('theme.css exposes zero-specificity public sveda tokens without cycling host variables', () => {
  assert.match(theme, /:where\(\.sveda-chat, sveda-chat\)/);
  assert.match(chat, /SvedaFrameTicks/);
  assert.match(theme, /--sveda-background:\s*0 0% 100%/);
  assert.match(theme, /--sveda-border:\s*42 18% 82%/);
  assert.match(theme, /\.sveda-chat-frame/);
  assert.match(theme, /border:\s*1px dashed hsl\(var\(--sveda-foreground\)\)/);
  assert.match(theme, /--font-serif:\s*Newsreader/);
  assert.match(appearance, /background: '0 0% 100%'/);
  assert.match(appearance, /export const mergeSvedaAppearance/);
  assert.match(appearance, /export const isSvedaAppearanceProvided/);
  assert.match(chat, /sveda-chat-frame/);
  assert.match(trigger, /tracking-\[0\.14em\]/);
  assert.match(trigger, /font-mono/);
  assert.match(theme, /--sveda-foreground:/);
  assert.match(theme, /--sveda-brand:/);
  assert.match(theme, /--sveda-radius:\s*0/);
  assert.match(theme, /:where\(\.sveda-chat\.dark, sveda-chat\.dark/);
  assert.match(theme, /--sveda-background:\s*30 12% 9%/);
  assert.match(theme, /\.sveda-chat-surface/);
  assert.match(theme, /body\.sveda-chat-immersive-mode/);
  assert.doesNotMatch(theme, /--background:\s*var\(--sveda-background\)/);
  assert.doesNotMatch(theme, /--foreground:\s*var\(--sveda-foreground\)/);
  assert.match(chat, /'sveda-chat'/);
});

test('portaled select and tooltip keep sveda tokens and popover backgrounds', () => {
  assert.match(selectContent, /sveda-chat/);
  assert.match(selectContent, /bg-popover/);
  assert.doesNotMatch(selectContent, /bg-input text-popover-foreground/);
  assert.match(tooltipContent, /sveda-chat/);
  assert.match(tooltipContent, /bg-popover/);
});

test('appearance helper injects radius and token declarations onto .sveda-chat', () => {
  assert.match(appearance, /SVEDA_APPEARANCE_STYLE_ID = 'sveda-appearance'/);
  assert.match(appearance, /export const applySvedaAppearance/);
  assert.match(appearance, /export const resolveSvedaAppearance/);
  assert.match(appearance, /SVEDA_APPEARANCE_PRESETS/);
  assert.match(appearance, /preset: 'lms'/);
  assert.match(appearance, /preset: 'ocean'/);
  assert.match(appearance, /preset: 'forest'/);
  assert.match(appearance, /preset: 'sunset'/);
  assert.match(appearance, /preset: 'sand'/);
  assert.match(appearance, /'166 72% 32%'/);
  assert.match(appearance, /\.sveda-chat\{/);
  assert.match(appearance, /\.dark \.sveda-chat,\.sveda-chat\.dark/);
  assert.match(appearance, /sanitizeSvedaTheme/);
  assert.match(appearance, /theme === 'dark'/);
  assert.match(appearance, /theme === 'light'/);
  assert.match(appearance, /SVEDA_LAUNCHER_ICON_IDS/);
  assert.match(appearanceVue, /useSvedaLauncher/);
  assert.match(appearance, /sanitizeSvedaLauncher/);
  assert.match(appearance, /sanitizeSvedaLauncherImage/);
  assert.match(appearance, /sanitizeSvedaChrome/);
  assert.match(appearanceVue, /useSvedaChrome/);
  assert.match(appearanceVue, /svedaChrome/);
  assert.match(appearance, /SVEDA_LAUNCHER_IMAGE_MAX_BYTES/);
});

test('thinking switch stays a pill with a raised thumb', () => {
  assert.match(switchUi, /rounded-full/);
  assert.match(switchUi, /shadow-inner/);
  assert.match(switchUi, /size === 'sm'/);
  assert.match(switchUi, /data-\[state=checked\]:bg-foreground/);
  assert.match(switchUi, /bg-background/);
  assert.doesNotMatch(switchUi, /rounded-\[var\(--sveda-radius\)\]/);
  assert.doesNotMatch(switchUi, /bg-brand-primary-purple-foreground/);
});

test('minimized trigger uses launcher label, icon, and image', () => {
  assert.match(trigger, /svedaLauncherIconComponent/);
  assert.match(chat, /chat.launcherLabel/);
  assert.match(chat, /chat.launcherIcon/);
  assert.match(chat, /chat.launcherImage/);
});

test('toast viewport stays inside the chat window', () => {
  const viewport = readFileSync(join(dir, 'ui/toast/ToastViewport.vue'), 'utf8');
  assert.match(viewport, /pointer-events-none/);
  assert.match(viewport, /absolute inset-x-0 bottom-0/);
  assert.match(viewport, /max-h-48/);
  assert.doesNotMatch(viewport, /max-h-screen/);
});

test('composer uses the dashed paper frame and chrome can hide model and thinking', () => {
  const input = readFileSync(join(dir, 'components/chat/ChatInput.vue'), 'utf8');
  const controls = readFileSync(join(dir, 'components/shell/SvedaModelControls.vue'), 'utf8');
  const model = readFileSync(join(dir, 'composables/useSvedaModel.ts'), 'utf8');
  assert.match(input, /sveda-chat-frame/);
  assert.match(input, /placeholder:font-mono/);
  assert.match(input, /border-t border-dashed border-foreground/);
  assert.match(controls, /useSvedaChrome/);
  assert.match(controls, /showModelSelect/);
  assert.match(controls, /showThinking/);
  assert.match(model, /chrome\.thinking && selectedModelSupportsThinking/);
});
