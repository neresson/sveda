<script setup>
import { useI18n } from 'vue-i18n';
import { useLocale } from '../i18n';

const { t, locale } = useI18n();
const { setLocale } = useLocale();

const languages = [
    { value: 'ru', label: 'RU' },
    { value: 'en', label: 'EN' },
];
</script>

<template>
    <div class="flex items-center gap-2" :aria-label="t('common.language')">
        <button
            v-for="language in languages"
            :key="language.value"
            type="button"
            class="veda-hover font-mono text-[11px] tracking-[0.16em]"
            :class="locale === language.value ? 'text-ink' : 'text-muted hover:text-ink'"
            @click="setLocale(language.value)"
        >
            {{ language.label }}
        </button>
    </div>
</template>
