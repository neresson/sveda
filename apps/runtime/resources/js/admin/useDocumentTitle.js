import { watchEffect } from 'vue';
import { useI18n } from 'vue-i18n';

export function useDocumentTitle(key) {
    const { t, locale } = useI18n();

    watchEffect(() => {
        locale.value;
        document.title = t(key);
    });
}
