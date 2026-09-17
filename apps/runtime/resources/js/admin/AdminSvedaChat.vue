<script setup>
import { Toaster, SvedaChat, SvedaI18nKey } from '@sveda-ai/vue';
import { computed, inject, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

const { t, locale } = useI18n();
const svedaI18n = inject(SvedaI18nKey, null);
const adminHref = inject('adminHref', ref(typeof window === 'undefined' ? '' : window.location.href));

watch(locale, (value) => svedaI18n?.setLocale(value), { immediate: true });

const brandName = computed(() => t('shell.brand'));
const pageUrl = computed(() => adminHref.value);
</script>

<template>
    <div class="sveda-chat sveda-chat-host relative">
        <SvedaChat
            :brand-name="brandName"
            :page-url="pageUrl"
        />
        <Toaster />
    </div>
</template>
