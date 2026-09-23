import { ref } from 'vue';
import { createI18n } from 'vue-i18n';
import en from './locales/en/index';
import ru from './locales/ru/index';

const STORAGE_KEY = 'locale';
const LOCALES = ['ru', 'en'];

const russianPluralRule = (choice, choicesLength) => {
    if (choice === 0) {
        return 0;
    }

    const teen = choice > 10 && choice < 20;
    const endsWithOne = choice % 10 === 1;

    if (!teen && endsWithOne) {
        return 1;
    }

    if (!teen && choice % 10 >= 2 && choice % 10 <= 4) {
        return 2;
    }

    return choicesLength < 4 ? 2 : 3;
};

const getStoredLocale = () => {
    if (typeof window === 'undefined') {
        return 'ru';
    }

    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (LOCALES.includes(stored)) {
        return stored;
    }

    return 'ru';
};

const applyDocumentLocale = (value) => {
    if (typeof document === 'undefined') {
        return;
    }

    document.documentElement.lang = value;
};

export const i18n = createI18n({
    legacy: false,
    locale: getStoredLocale(),
    fallbackLocale: 'ru',
    messages: { en, ru },
    pluralRules: {
        ru: russianPluralRule,
    },
    globalInjection: true,
});

applyDocumentLocale(i18n.global.locale.value);

const setCookie = (name, value, days = 365) => {
    if (typeof document === 'undefined') {
        return;
    }

    const maxAge = days * 24 * 60 * 60;
    document.cookie = `${name}=${value};path=/;max-age=${maxAge};SameSite=Lax`;
};

export function useLocale() {
    const locale = ref(getStoredLocale());

    function setLocale(value) {
        if (!LOCALES.includes(value)) {
            return;
        }

        locale.value = value;
        i18n.global.locale.value = value;
        window.localStorage.setItem(STORAGE_KEY, value);
        setCookie(STORAGE_KEY, value);
        applyDocumentLocale(value);
    }

    return {
        locale,
        setLocale,
    };
}

export function installI18n(app) {
    app.use(i18n);
}
