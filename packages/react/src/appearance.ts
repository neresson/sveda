import {
  applySvedaAppearance as applySvedaAppearanceDom,
  resolveSvedaAppearance,
  sanitizeSvedaChrome,
  sanitizeSvedaLauncher,
  type SvedaAppearance,
  type SvedaAppearanceChrome,
  type SvedaAppearanceLauncher,
} from '@sveda-ai/chat';

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

let launcherState: SvedaAppearanceLauncher = sanitizeSvedaLauncher(null);
let chromeState: SvedaAppearanceChrome = sanitizeSvedaChrome(null);
const appearanceListeners = new Set<() => void>();

const notifyAppearance = () => {
  for (const listener of appearanceListeners) {
    listener();
  }
};

export const subscribeSvedaAppearance = (listener: () => void): (() => void) => {
  appearanceListeners.add(listener);
  return () => {
    appearanceListeners.delete(listener);
  };
};

export const getSvedaLauncherSnapshot = (): SvedaAppearanceLauncher => launcherState;

export const getSvedaChromeSnapshot = (): SvedaAppearanceChrome => chromeState;

export const useSvedaLauncher = (): SvedaAppearanceLauncher => launcherState;

export const useSvedaChrome = (): SvedaAppearanceChrome => chromeState;

export const svedaLauncher = launcherState;

export const svedaChrome = chromeState;

export const applySvedaAppearance = (appearance: SvedaAppearance | null | undefined): void => {
  const resolved = resolveSvedaAppearance(appearance);
  launcherState = sanitizeSvedaLauncher(resolved?.launcher);
  chromeState = sanitizeSvedaChrome(resolved?.chrome);
  applySvedaAppearanceDom(appearance);
  notifyAppearance();
};
