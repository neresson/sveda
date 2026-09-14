<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import AuthFrame from './AuthFrame.vue';
import { useDocumentTitle } from './useDocumentTitle';

const props = defineProps({
    csrf: { type: String, required: true },
    action: { type: String, required: true },
    error: { type: String, default: '' },
});

const { t } = useI18n();
useDocumentTitle('login.document_title');

const errorMessage = computed(() => (props.error ? t('login.invalid') : ''));
</script>

<template>
    <AuthFrame index="01" :brand="t('shell.brand')" :footer="t('login.footer')" variant="login">
        <template #title>
            <h1 class="text-7xl font-bold tracking-tighter max-md:text-5xl max-sm:text-4xl">{{ t('login.title') }}</h1>
            <p class="mt-4 font-serif text-xl text-muted max-lg:text-base">{{ t('login.subtitle') }}</p>
        </template>

        <p v-if="errorMessage" class="mb-6 font-mono text-sm text-ink">{{ errorMessage }}</p>

        <form method="post" :action="action" class="flex flex-col gap-6">
            <input type="hidden" name="_token" :value="csrf">
            <label class="flex flex-col gap-2">
                <span class="font-mono text-[11px] tracking-[0.16em]">{{ t('login.admin_key') }}</span>
                <input
                    id="key"
                    name="key"
                    type="password"
                    autocomplete="current-password"
                    required
                    class="border border-ink bg-canvas px-4 py-3.5 font-mono text-sm outline-none"
                >
            </label>
            <button type="submit" class="bg-ink px-4 py-4 font-semibold text-canvas">
                {{ t('login.submit') }}
            </button>
            <p class="text-xs leading-relaxed text-muted">{{ t('login.hint') }}</p>
        </form>
    </AuthFrame>
</template>
