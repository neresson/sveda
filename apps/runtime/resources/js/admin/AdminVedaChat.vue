<script setup>
import { Toaster, VedaChat, VedaI18nKey } from '@veda-ai/vue';
import { computed, inject, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

const { t, locale } = useI18n();
const vedaI18n = inject(VedaI18nKey, null);
const adminHref = inject('adminHref', ref(typeof window === 'undefined' ? '' : window.location.href));

watch(locale, (value) => vedaI18n?.setLocale(value), { immediate: true });

const brandName = computed(() => t('shell.brand'));
const pageUrl = computed(() => adminHref.value);
</script>

<template>
    <div class="veda-chat veda-chat-host">
        <VedaChat
            :brand-name="brandName"
            :page-url="pageUrl"
        />
        <Toaster />
    </div>
</template>
