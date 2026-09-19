import { reactive } from 'vue';

export const SVEDA_APPEARANCE_STYLE_ID = 'sveda-appearance';

export const SVEDA_TOKEN_KEYS = [
  'background',
  'foreground',
  'card',
  'card_foreground',
  'popover',
  'popover_foreground',
  'primary',
  'primary_foreground',
  'secondary',
  'secondary_foreground',
  'muted',
  'muted_foreground',
  'accent',
  'accent_foreground',
  'destructive',
  'destructive_foreground',
  'border',
  'input',
  'ring',
  'brand',
  'brand_foreground',
] as const;

export type SvedaTokenKey = (typeof SVEDA_TOKEN_KEYS)[number];

export type SvedaAppearanceTokens = Record<SvedaTokenKey, string>;

export type SvedaAppearanceTheme = 'light' | 'dark';

export const SVEDA_LAUNCHER_ICON_IDS = [
  'sparkles',
  'message-circle',
  'message-square',
  'bot',
  'bot-message-square',
  'brain',
  'zap',
  'star',
  'heart',
  'circle-help',
  'rocket',
  'gem',
] as const;

export const SVEDA_DEFAULT_LAUNCHER_ICON = 'sparkles';

export const SVEDA_LAUNCHER_IMAGE_MAX_BYTES = 262144;

export const SVEDA_LAUNCHER_IMAGE_MAX_CHARS = 360000;

export type SvedaLauncherIconId = (typeof SVEDA_LAUNCHER_ICON_IDS)[number];

export type SvedaAppearanceLauncher = {
  label: string;
  icon: SvedaLauncherIconId;
  image: string;
};

export type SvedaAppearanceChrome = {
  modelSelect: boolean;
  thinking: boolean;
};

export type SvedaAppearance = {
  preset?: string;
  radius?: string;
  theme?: SvedaAppearanceTheme;
  tokens?: Partial<SvedaAppearanceTokens> | Record<string, string>;
  dark_tokens?: Partial<SvedaAppearanceTokens> | Record<string, string>;
  launcher?: Partial<SvedaAppearanceLauncher> | { label?: string; icon?: string; image?: string };
  chrome?: Partial<SvedaAppearanceChrome>;
};

export const SVEDA_APPEARANCE_PRESET_IDS = ['default', 'lms', 'ocean', 'forest', 'sunset', 'sand'] as const;

export type SvedaAppearancePresetId = (typeof SVEDA_APPEARANCE_PRESET_IDS)[number];

const LANDING_LIGHT: SvedaAppearanceTokens = {
  background: '0 0% 100%',
  foreground: '0 0% 4%',
  card: '0 0% 100%',
  card_foreground: '0 0% 4%',
  popover: '0 0% 100%',
  popover_foreground: '0 0% 4%',
  primary: '0 0% 4%',
  primary_foreground: '0 0% 100%',
  secondary: '40 20% 90%',
  secondary_foreground: '0 0% 4%',
  muted: '40 22% 90%',
  muted_foreground: '40 6% 41%',
  accent: '40 22% 90%',
  accent_foreground: '0 0% 4%',
  destructive: '0 84.2% 60.2%',
  destructive_foreground: '0 0% 98%',
  border: '42 18% 82%',
  input: '40 22% 92%',
  ring: '0 0% 4%',
  brand: '0 0% 4%',
  brand_foreground: '0 0% 100%',
};

const LANDING_DARK: SvedaAppearanceTokens = {
  background: '30 12% 9%',
  foreground: '40 24% 94%',
  card: '30 10% 12%',
  card_foreground: '40 24% 94%',
  popover: '30 10% 12%',
  popover_foreground: '40 24% 94%',
  primary: '40 24% 94%',
  primary_foreground: '30 12% 9%',
  secondary: '30 8% 16%',
  secondary_foreground: '40 24% 94%',
  muted: '30 8% 16%',
  muted_foreground: '36 10% 62%',
  accent: '30 8% 16%',
  accent_foreground: '40 24% 94%',
  destructive: '0 62.8% 30.6%',
  destructive_foreground: '40 24% 94%',
  border: '30 8% 22%',
  input: '30 8% 16%',
  ring: '40 24% 94%',
  brand: '40 24% 94%',
  brand_foreground: '30 12% 9%',
};

const NEUTRAL_LIGHT: SvedaAppearanceTokens = { ...LANDING_LIGHT };

const NEUTRAL_DARK: SvedaAppearanceTokens = { ...LANDING_DARK };

export const SVEDA_APPEARANCE_PRESETS: Record<SvedaAppearancePresetId, Required<Pick<SvedaAppearance, 'preset' | 'radius'>> & {
  tokens: SvedaAppearanceTokens;
  dark_tokens: SvedaAppearanceTokens;
}> = {
  default: {
    preset: 'default',
    radius: '0px',
    tokens: { ...NEUTRAL_LIGHT },
    dark_tokens: { ...NEUTRAL_DARK },
  },
  lms: {
    preset: 'lms',
    radius: '0px',
    tokens: {
      ...NEUTRAL_LIGHT,
      accent: '275 60% 96%',
      ring: '275 96% 52%',
      brand: '275 96% 52%',
      brand_foreground: '0 0% 98%',
    },
    dark_tokens: {
      ...NEUTRAL_DARK,
      accent: '275 40% 18%',
      ring: '275 100% 62%',
      brand: '275 100% 42%',
      brand_foreground: '0 0% 98%',
    },
  },
  ocean: {
    preset: 'ocean',
    radius: '0px',
    tokens: {
      ...NEUTRAL_LIGHT,
      background: '214 40% 98%',
      accent: '214 80% 96%',
      ring: '221 83% 53%',
      brand: '221 83% 53%',
      brand_foreground: '0 0% 98%',
    },
    dark_tokens: {
      ...NEUTRAL_DARK,
      accent: '217 40% 18%',
      ring: '213 94% 68%',
      brand: '213 94% 68%',
      brand_foreground: '222.2 47.4% 11.2%',
    },
  },
  forest: {
    preset: 'forest',
    radius: '0px',
    tokens: {
      ...NEUTRAL_LIGHT,
      background: '168 25% 98%',
      card: '150 20% 99%',
      popover: '150 20% 99%',
      accent: '166 30% 94%',
      ring: '166 72% 32%',
      brand: '166 72% 32%',
      brand_foreground: '0 0% 98%',
    },
    dark_tokens: {
      ...NEUTRAL_DARK,
      accent: '166 28% 18%',
      ring: '166 50% 52%',
      brand: '166 50% 52%',
      brand_foreground: '222.2 47.4% 11.2%',
    },
  },
  sunset: {
    preset: 'sunset',
    radius: '0px',
    tokens: {
      ...NEUTRAL_LIGHT,
      background: '28 45% 98%',
      card: '30 50% 99%',
      popover: '30 50% 99%',
      accent: '20 70% 95%',
      ring: '16 82% 50%',
      brand: '16 82% 50%',
      brand_foreground: '0 0% 98%',
    },
    dark_tokens: {
      ...NEUTRAL_DARK,
      accent: '16 40% 18%',
      ring: '18 85% 62%',
      brand: '18 85% 62%',
      brand_foreground: '222.2 47.4% 11.2%',
    },
  },
  sand: {
    preset: 'sand',
    radius: '0px',
    tokens: {
      ...NEUTRAL_LIGHT,
      background: '40 33% 97%',
      card: '40 40% 99%',
      popover: '40 40% 99%',
      muted: '36 24% 93%',
      accent: '36 30% 93%',
      border: '36 18% 86%',
      input: '36 22% 94%',
      ring: '28 35% 24%',
      brand: '28 35% 24%',
      brand_foreground: '40 33% 97%',
    },
    dark_tokens: {
      ...NEUTRAL_DARK,
      background: '30 12% 11%',
      card: '30 10% 14%',
      popover: '30 10% 14%',
      accent: '30 12% 18%',
      border: '30 10% 20%',
      input: '30 10% 18%',
      ring: '36 35% 72%',
      brand: '36 35% 72%',
      brand_foreground: '30 12% 11%',
    },
  },
};

const isPresetId = (value: unknown): value is SvedaAppearancePresetId =>
  typeof value === 'string' && (SVEDA_APPEARANCE_PRESET_IDS as readonly string[]).includes(value);

const HSL_PATTERN = /^\d{1,3}(?:\.\d+)?\s+\d{1,3}(?:\.\d+)?%\s+\d{1,3}(?:\.\d+)?%$/;
const RADIUS_PATTERN = /^\d+(?:\.\d+)?(?:px|rem|em)$/;

const tokenCssName = (key: string): string => `--sveda-${key.replace(/_/g, '-')}`;

export const isSvedaHsl = (value: unknown): value is string =>
  typeof value === 'string' && HSL_PATTERN.test(value.trim());

export const sanitizeSvedaTheme = (value: unknown): SvedaAppearanceTheme | undefined =>
  value === 'light' || value === 'dark' ? value : undefined;

const isLauncherIconId = (value: unknown): value is SvedaLauncherIconId =>
  typeof value === 'string' && (SVEDA_LAUNCHER_ICON_IDS as readonly string[]).includes(value);

export const sanitizeSvedaLauncherImage = (value: unknown): string => {
  if (typeof value !== 'string') {
    return '';
  }

  const trimmed = value.trim();
  if (trimmed === '') {
    return '';
  }

  const dataMatch = trimmed.match(/^data:image\/(png|jpeg|jpg|webp|gif);base64,([A-Za-z0-9+/=\s]+)$/i);
  if (dataMatch) {
    const payload = dataMatch[2].replace(/\s+/g, '');
    const detected = dataMatch[1].toLowerCase() === 'jpg' ? 'jpeg' : dataMatch[1].toLowerCase();
    const normalized = `data:image/${detected};base64,${payload}`;
    if (normalized.length > SVEDA_LAUNCHER_IMAGE_MAX_CHARS) {
      return '';
    }

    return normalized;
  }

  if (/^https?:\/\//i.test(trimmed) && trimmed.length <= 2048) {
    try {
      const url = new URL(trimmed);
      if (url.protocol === 'http:' || url.protocol === 'https:') {
        return trimmed;
      }
    } catch {
      return '';
    }
  }

  if (
    trimmed.startsWith('/') &&
    !trimmed.startsWith('//') &&
    !trimmed.includes('..') &&
    !trimmed.includes('\\') &&
    !/[\s<>"']/.test(trimmed) &&
    trimmed.length <= 2048
  ) {
    return trimmed;
  }

  return '';
};

export const sanitizeSvedaLauncher = (launcher: SvedaAppearance['launcher'] | null | undefined): SvedaAppearanceLauncher => {
  const label = typeof launcher?.label === 'string' ? launcher.label.trim().replace(/\s+/g, ' ').slice(0, 64) : '';
  const icon = isLauncherIconId(launcher?.icon) ? launcher.icon : SVEDA_DEFAULT_LAUNCHER_ICON;

  return { label, icon, image: sanitizeSvedaLauncherImage(launcher?.image) };
};

export const svedaLauncher = reactive<SvedaAppearanceLauncher>(sanitizeSvedaLauncher(null));

export const useSvedaLauncher = (): SvedaAppearanceLauncher => svedaLauncher;

const readChromeFlag = (value: unknown, fallback: boolean): boolean =>
  typeof value === 'boolean' ? value : fallback;

export const sanitizeSvedaChrome = (
  chrome: SvedaAppearance['chrome'] | Record<string, unknown> | null | undefined,
): SvedaAppearanceChrome => {
  const record = chrome && typeof chrome === 'object' ? (chrome as Record<string, unknown>) : null;

  return {
    modelSelect: readChromeFlag(record?.modelSelect ?? record?.model_select, true),
    thinking: readChromeFlag(record?.thinking, true),
  };
};

export const svedaChrome = reactive<SvedaAppearanceChrome>(sanitizeSvedaChrome(null));

export const useSvedaChrome = (): SvedaAppearanceChrome => svedaChrome;

export const sanitizeSvedaRadius = (value: unknown): string | null => {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return `${Math.max(0, Math.min(64, Math.round(value)))}px`;
  }

  if (typeof value !== 'string') {
    return null;
  }

  const trimmed = value.trim();
  if (trimmed === '0') {
    return '0px';
  }

  if (!RADIUS_PATTERN.test(trimmed)) {
    return null;
  }

  return trimmed;
};

export const parseRadiusPx = (value: unknown): number => {
  const raw = String(value ?? '').trim();
  const match = raw.match(/^([\d.]+)(px|rem|em)$/);
  if (!match) {
    return raw === '0' ? 0 : 0;
  }

  const amount = Number(match[1]);
  if (!Number.isFinite(amount)) {
    return 0;
  }

  if (match[2] === 'px') {
    return Math.max(0, Math.min(64, amount));
  }

  return Math.max(0, Math.min(64, amount * 16));
};

export const formatRadiusPx = (px: number): string => `${Math.round(Math.max(0, Math.min(64, px)))}px`;

export const hslToHex = (hsl: string): string => {
  const match = String(hsl)
    .trim()
    .match(/^([\d.]+)\s+([\d.]+)%\s+([\d.]+)%$/);
  if (!match) {
    return '#000000';
  }

  const h = (((Number(match[1]) % 360) + 360) % 360) / 360;
  const s = Math.min(100, Math.max(0, Number(match[2]))) / 100;
  const l = Math.min(100, Math.max(0, Number(match[3]))) / 100;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number): string => {
    const k = (n + h * 12) % 12;
    const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);

    return Math.round(255 * color)
      .toString(16)
      .padStart(2, '0');
  };

  return `#${f(0)}${f(8)}${f(4)}`;
};

export const hexToHsl = (hex: string): string => {
  const raw = hex.replace('#', '').trim();
  const full = raw.length === 3 ? raw.split('').map((char) => char + char).join('') : raw;
  if (!/^[\da-f]{6}$/i.test(full)) {
    return '0 0% 0%';
  }

  const r = parseInt(full.slice(0, 2), 16) / 255;
  const g = parseInt(full.slice(2, 4), 16) / 255;
  const b = parseInt(full.slice(4, 6), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      default:
        h = (r - g) / d + 4;
        break;
    }
    h *= 60;
  }

  return `${Math.round(h)} ${Math.round(s * 100)}% ${Math.round(l * 100)}%`;
};

const unwrapHsl = (value: unknown): string | null => {
  if (typeof value !== 'string') {
    return null;
  }

  let next = value.trim();
  next = next.replace(/^hsla?\(/i, '');
  next = next.replace(/\)$/, '').trim();

  return HSL_PATTERN.test(next) ? next : null;
};

const overlayTokens = (tokens: SvedaAppearance['tokens']): Partial<SvedaAppearanceTokens> => {
  const clean: Partial<SvedaAppearanceTokens> = {};
  if (!tokens || typeof tokens !== 'object') {
    return clean;
  }

  for (const key of SVEDA_TOKEN_KEYS) {
    const value = unwrapHsl(tokens[key]);
    if (value) {
      clean[key] = value;
    }
  }

  return clean;
};

const mergeTokens = (base: SvedaAppearanceTokens, overlay: Partial<SvedaAppearanceTokens>): SvedaAppearanceTokens => {
  const merged = { ...base };
  for (const key of SVEDA_TOKEN_KEYS) {
    const value = overlay[key];
    if (value) {
      merged[key] = value;
    }
  }

  return merged;
};

const sameTokens = (left: SvedaAppearanceTokens, right: SvedaAppearanceTokens): boolean =>
  SVEDA_TOKEN_KEYS.every((key) => left[key] === right[key]);

export const resolveSvedaAppearance = (appearance: SvedaAppearance | null | undefined): SvedaAppearance | null => {
  if (!appearance || typeof appearance !== 'object') {
    return null;
  }

  const legacyRounded = appearance.preset === 'rounded';
  const namedId = isPresetId(appearance.preset)
    ? appearance.preset
    : legacyRounded
      ? 'forest'
      : 'default';
  const named = SVEDA_APPEARANCE_PRESETS[namedId];
  const tokenOverlay = overlayTokens(appearance.tokens);
  const darkOverlay = overlayTokens(appearance.dark_tokens);
  const tokens =
    Object.keys(tokenOverlay).length === 0 ? named.tokens : mergeTokens(named.tokens, tokenOverlay);
  const darkTokens =
    Object.keys(darkOverlay).length === 0 ? named.dark_tokens : mergeTokens(named.dark_tokens, darkOverlay);
  const radius = Object.prototype.hasOwnProperty.call(appearance, 'radius')
    ? (sanitizeSvedaRadius(appearance.radius) ?? '0px')
    : legacyRounded
      ? '20px'
      : named.radius;
  const customTokens = !sameTokens(tokens, named.tokens) || !sameTokens(darkTokens, named.dark_tokens);

  const theme = sanitizeSvedaTheme(appearance.theme);
  const launcher = sanitizeSvedaLauncher(appearance.launcher);
  const chrome = sanitizeSvedaChrome(appearance.chrome);

  if (appearance.preset === 'custom' || customTokens) {
    return {
      preset: 'custom',
      radius,
      ...(theme ? { theme } : {}),
      tokens,
      dark_tokens: darkTokens,
      launcher,
      chrome,
    };
  }

  return {
    preset: named.preset,
    radius,
    ...(theme ? { theme } : {}),
    tokens: named.tokens,
    dark_tokens: named.dark_tokens,
    launcher,
    chrome,
  };
};

const declarations = (tokens: Record<string, string> | undefined, radius: string): string => {
  const parts = [`--sveda-radius:${radius}`];
  for (const key of SVEDA_TOKEN_KEYS) {
    const value = unwrapHsl(tokens?.[key]);
    if (!value) {
      continue;
    }

    parts.push(`${tokenCssName(key)}:${value}`);
  }

  return parts.join(';');
};

export const buildAppearanceCss = (appearance: SvedaAppearance | null | undefined): string => {
  const resolved = resolveSvedaAppearance(appearance);
  if (!resolved) {
    return '';
  }

  const radius = sanitizeSvedaRadius(resolved.radius) ?? '0px';
  const light = declarations(resolved.tokens as Record<string, string> | undefined, radius);
  const dark = declarations(resolved.dark_tokens as Record<string, string> | undefined, radius);
  const theme = sanitizeSvedaTheme(resolved.theme);

  if (theme === 'dark') {
    return `.sveda-chat{${dark}}`;
  }

  if (theme === 'light') {
    return `.sveda-chat{${light}}`;
  }

  return `.sveda-chat{${light}}.dark .sveda-chat,.sveda-chat.dark{${dark}}`;
};

const isPlainAppearance = (value: unknown): value is SvedaAppearance =>
  Boolean(value) && typeof value === 'object' && !Array.isArray(value);

export const isSvedaAppearanceProvided = (value: unknown): value is SvedaAppearance =>
  isPlainAppearance(value) && Object.keys(value).length > 0;

const overlayRecord = <T extends Record<string, unknown>>(
  admin: T | null | undefined,
  host: T | null | undefined,
): T | undefined => {
  if (!host && !admin) {
    return undefined;
  }

  return { ...(admin ?? {}), ...(host ?? {}) } as T;
};

export const mergeSvedaAppearance = (
  admin: SvedaAppearance | null | undefined,
  host: SvedaAppearance | null | undefined,
): SvedaAppearance | null => {
  const adminProvided = isSvedaAppearanceProvided(admin);
  const hostProvided = isSvedaAppearanceProvided(host);
  if (!adminProvided && !hostProvided) {
    return null;
  }

  if (!hostProvided) {
    return { ...(admin as SvedaAppearance) };
  }

  if (!adminProvided) {
    return { ...(host as SvedaAppearance) };
  }

  const base = admin as SvedaAppearance;
  const overlay = host as SvedaAppearance;
  const merged: SvedaAppearance = { ...base, ...overlay };
  const launcher = overlayRecord(base.launcher as Record<string, unknown> | undefined, overlay.launcher as Record<string, unknown> | undefined);
  const chrome = overlayRecord(base.chrome as Record<string, unknown> | undefined, overlay.chrome as Record<string, unknown> | undefined);

  if (launcher) {
    merged.launcher = launcher;
  }

  if (chrome) {
    merged.chrome = chrome;
  }

  const hostSetsPreset = Object.prototype.hasOwnProperty.call(overlay, 'preset');
  const hostSetsTokens = Object.prototype.hasOwnProperty.call(overlay, 'tokens');
  const hostSetsDarkTokens = Object.prototype.hasOwnProperty.call(overlay, 'dark_tokens');

  if (hostSetsPreset && !hostSetsTokens) {
    delete merged.tokens;
  } else if (base.tokens || overlay.tokens) {
    merged.tokens = overlayRecord(base.tokens as Record<string, unknown> | undefined, overlay.tokens as Record<string, unknown> | undefined);
  }

  if (hostSetsPreset && !hostSetsDarkTokens) {
    delete merged.dark_tokens;
  } else if (base.dark_tokens || overlay.dark_tokens) {
    merged.dark_tokens = overlayRecord(
      base.dark_tokens as Record<string, unknown> | undefined,
      overlay.dark_tokens as Record<string, unknown> | undefined,
    );
  }

  return merged;
};

export const applySvedaAppearance = (appearance: SvedaAppearance | null | undefined): void => {
  const resolved = resolveSvedaAppearance(appearance);
  const launcher = sanitizeSvedaLauncher(resolved?.launcher);
  svedaLauncher.label = launcher.label;
  svedaLauncher.icon = launcher.icon;
  svedaLauncher.image = launcher.image;

  const chrome = sanitizeSvedaChrome(resolved?.chrome);
  svedaChrome.modelSelect = chrome.modelSelect;
  svedaChrome.thinking = chrome.thinking;

  if (typeof document === 'undefined') {
    return;
  }

  let style = document.getElementById(SVEDA_APPEARANCE_STYLE_ID) as HTMLStyleElement | null;
  if (!style) {
    style = document.createElement('style');
    style.id = SVEDA_APPEARANCE_STYLE_ID;
    document.head.appendChild(style);
  }

  style.textContent = buildAppearanceCss(appearance);
};
