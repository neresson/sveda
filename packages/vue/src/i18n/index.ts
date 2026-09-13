import { inject, reactive, type App, type InjectionKey } from 'vue';

export type VedaMessages = Record<string, string>;

export interface VedaI18n {
  readonly locale: string;
  t(key: string, params?: Record<string, unknown>): string;
  setLocale(locale: string): void;
  extend(locale: string, messages: VedaMessages): void;
}

export interface VedaI18nOptions {
  locale?: string;
  fallbackLocale?: string;
  messages?: Record<string, VedaMessages>;
}

export const VedaI18nKey: InjectionKey<VedaI18n> = Symbol('veda-i18n');

export function createVedaI18n(options: VedaI18nOptions = {}): VedaI18n {
  const state = reactive({
    locale: options.locale ?? 'en',
    fallbackLocale: options.fallbackLocale ?? 'en',
    messages: { ...(options.messages ?? {}) } as Record<string, VedaMessages>,
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
    extend(locale: string, messages: VedaMessages): void {
      state.messages[locale] = { ...(state.messages[locale] ?? {}), ...messages };
    },
  };
}

export function installVedaI18n(app: App, i18n: VedaI18n): void {
  app.provide(VedaI18nKey, i18n);
}

export function useVedaT(): (key: string, params?: Record<string, unknown>) => string {
  const i18n = inject(VedaI18nKey);

  if (!i18n) {
    return (key: string) => key;
  }

  return (key, params) => i18n.t(key, params);
}
