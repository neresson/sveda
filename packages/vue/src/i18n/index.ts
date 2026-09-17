import { inject, reactive, type App, type InjectionKey } from 'vue';

export type SvedaMessages = Record<string, string>;

export interface SvedaI18n {
  readonly locale: string;
  t(key: string, params?: Record<string, unknown>): string;
  setLocale(locale: string): void;
  extend(locale: string, messages: SvedaMessages): void;
}

export interface SvedaI18nOptions {
  locale?: string;
  fallbackLocale?: string;
  messages?: Record<string, SvedaMessages>;
}

export const SvedaI18nKey: InjectionKey<SvedaI18n> = Symbol('sveda-i18n');

export function createSvedaI18n(options: SvedaI18nOptions = {}): SvedaI18n {
  const state = reactive({
    locale: options.locale ?? 'en',
    fallbackLocale: options.fallbackLocale ?? 'en',
    messages: { ...(options.messages ?? {}) } as Record<string, SvedaMessages>,
  });

  const interpolate = (template: string, params?: Record<string, unknown>): string => {
    if (!params) {
      return template;
    }

    return template.replace(/\{(\w+)\}/g, (match, name: string) =>
      params[name] === undefined || params[name] === null ? match : String(params[name])
    );
  };

  return {
    get locale() {
      return state.locale;
    },
    t(key: string, params?: Record<string, unknown>): string {
      const message =
        state.messages[state.locale]?.[key] ?? state.messages[state.fallbackLocale]?.[key];

      if (message === undefined) {
        return key;
      }

      return interpolate(message, params);
    },
    setLocale(locale: string): void {
      state.locale = locale;
    },
    extend(locale: string, messages: SvedaMessages): void {
      state.messages[locale] = { ...(state.messages[locale] ?? {}), ...messages };
    },
  };
}

export function installSvedaI18n(app: App, i18n: SvedaI18n): void {
  app.provide(SvedaI18nKey, i18n);
}

export function useSvedaT(): (key: string, params?: Record<string, unknown>) => string {
  const i18n = inject(SvedaI18nKey);

  if (!i18n) {
    return (key: string) => key;
  }

  return (key, params) => i18n.t(key, params);
}
