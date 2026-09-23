import {
  applySvedaAppearance as applySvedaAppearanceDom,
  resolveSvedaAppearance,
  sanitizeSvedaChrome,
  sanitizeSvedaLauncher,
  type SvedaAppearance,
  type SvedaAppearanceChrome,
  type SvedaAppearanceLauncher,
} from '@sveda-ai/chat';
import { reactive } from 'vue';

export {
  SVEDA_APPEARANCE_STYLE_ID,
  SVEDA_TOKEN_KEYS,
  SVEDA_LAUNCHER_ICON_IDS,
  SVEDA_DEFAULT_LAUNCHER_ICON,
  SVEDA_LAUNCHER_IMAGE_MAX_BYTES,
  SVEDA_LAUNCHER_IMAGE_MAX_CHARS,
  SVEDA_APPEARANCE_PRESET_IDS,
  SVEDA_APPEARANCE_PRESETS,
  isSvedaHsl,
  sanitizeSvedaTheme,
  sanitizeSvedaLauncherImage,
  sanitizeSvedaLauncher,
  sanitizeSvedaChrome,
  sanitizeSvedaRadius,
  parseRadiusPx,
  formatRadiusPx,
  hslToHex,
  hexToHsl,
  resolveSvedaAppearance,
  buildAppearanceCss,
  isSvedaAppearanceProvided,
  mergeSvedaAppearance,
} from '@sveda-ai/chat';
export type {
  SvedaTokenKey,
  SvedaAppearanceTokens,
  SvedaAppearanceTheme,
  SvedaLauncherIconId,
  SvedaAppearanceLauncher,
  SvedaAppearanceChrome,
  SvedaAppearance,
  SvedaAppearancePresetId,
} from '@sveda-ai/chat';

export const svedaLauncher = reactive<SvedaAppearanceLauncher>(sanitizeSvedaLauncher(null));

export const useSvedaLauncher = (): SvedaAppearanceLauncher => svedaLauncher;

export const svedaChrome = reactive<SvedaAppearanceChrome>(sanitizeSvedaChrome(null));

export const useSvedaChrome = (): SvedaAppearanceChrome => svedaChrome;

export const applySvedaAppearance = (appearance: SvedaAppearance | null | undefined): void => {
  const resolved = resolveSvedaAppearance(appearance);
  const launcher = sanitizeSvedaLauncher(resolved?.launcher);
  svedaLauncher.label = launcher.label;
  svedaLauncher.icon = launcher.icon;
  svedaLauncher.image = launcher.image;

  const chrome = sanitizeSvedaChrome(resolved?.chrome);
  svedaChrome.modelSelect = chrome.modelSelect;
  svedaChrome.thinking = chrome.thinking;

  applySvedaAppearanceDom(appearance);
};
