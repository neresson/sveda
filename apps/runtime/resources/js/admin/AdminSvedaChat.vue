<script setup>
import { Toaster, SvedaChat, SvedaI18nKey, useSvedaChat, useSvedaClient } from '@sveda-ai/vue';
import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { pageFromAdminToolResult } from './adminOperator';

const { t, locale } = useI18n();
const svedaI18n = inject(SvedaI18nKey, null);
const adminHref = inject('adminHref', ref(typeof window === 'undefined' ? '' : window.location.href));
const adminPage = inject('adminPage', ref('dashboard'));
const reloadAdmin = inject('reloadAdmin', null);
const client = useSvedaClient();
const { currentChat } = useSvedaChat();
const seenAdminTools = new Set();
let unregisterAdminContext = null;

watch(locale, (value) => svedaI18n?.setLocale(value), { immediate: true });

onMounted(() => {
    unregisterAdminContext = client.contextRegistry.register('admin', () => ({
        page: adminPage?.value ?? 'dashboard',
    }));
});

onUnmounted(() => {
    unregisterAdminContext?.();
});

watch(
    () => currentChat.value?.messages,
    (messages) => {
        if (!reloadAdmin) {
            return;
        }

        for (const message of messages ?? []) {
            for (const part of message.parts ?? []) {
                const sync = pageFromAdminToolResult(part);
                if (!sync || seenAdminTools.has(sync.id)) {
                    continue;
                }

                seenAdminTools.add(sync.id);
                void reloadAdmin(sync);
            }
        }
    },
    { deep: true },
);

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
