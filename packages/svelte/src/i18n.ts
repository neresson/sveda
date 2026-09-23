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

export function createSvedaI18n(options: SvedaI18nOptions = {}): SvedaI18n {
  let locale = options.locale ?? 'en';
  const fallbackLocale = options.fallbackLocale ?? 'en';
  const messages: Record<string, SvedaMessages> = { ...(options.messages ?? {}) };

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
      return locale;
    },
    t(key: string, params?: Record<string, unknown>): string {
      const message = messages[locale]?.[key] ?? messages[fallbackLocale]?.[key];

      if (message === undefined) {
        return key;
      }

      return interpolate(message, params);
    },
    setLocale(next: string): void {
      locale = next;
    },
    extend(nextLocale: string, localeMessages: SvedaMessages): void {
      messages[nextLocale] = { ...(messages[nextLocale] ?? {}), ...localeMessages };
    },
  };
}
