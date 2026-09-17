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
useDocumentTitle('setup.document_title');

const errorMessage = computed(() => {
    if (!props.error) {
        return '';
    }

    if (['required', 'min', 'confirmed'].includes(props.error)) {
        return t(`setup.${props.error}`);
    }

    return t('setup.required');
});
</script>

<template>
    <AuthFrame index="00" :brand="t('shell.brand_setup')" :footer="t('setup.footer')" variant="setup">
        <template #title>
            <h1 class="text-7xl font-bold tracking-tighter max-md:text-5xl max-sm:text-4xl">
                {{ t('setup.title_lead') }}<br>{{ t('setup.title_tail') }}
            </h1>
            <p class="mt-4 font-serif text-xl text-muted max-lg:text-base">{{ t('setup.subtitle') }}</p>
        </template>

        <p v-if="errorMessage" class="mb-6 font-mono text-sm text-ink">{{ errorMessage }}</p>

        <form method="post" :action="action" class="flex flex-col gap-6">
            <input type="hidden" name="_token" :value="csrf">
            <label class="flex flex-col gap-2">
                <span class="font-mono text-[11px] tracking-[0.16em]">{{ t('setup.admin_key') }}</span>
                <input
                    id="key"
                    name="key"
                    type="password"
                    autocomplete="new-password"
                    minlength="16"
                    required
                    class="border border-ink bg-canvas px-4 py-3.5 font-mono text-sm outline-none"
                >
            </label>
            <label class="flex flex-col gap-2">
                <span class="font-mono text-[11px] tracking-[0.16em]">{{ t('setup.confirm_key') }}</span>
                <input
                    id="key_confirmation"
                    name="key_confirmation"
                    type="password"
                    autocomplete="new-password"
                    minlength="16"
                    required
                    class="border border-ink bg-canvas px-4 py-3.5 font-mono text-sm outline-none"
                >
            </label>
            <button type="submit" class="sveda-hover bg-ink px-4 py-4 font-semibold text-canvas hover:bg-ink/80">
                {{ t('setup.submit') }}
            </button>
            <p class="text-xs leading-relaxed text-muted">{{ t('setup.hint') }}</p>
        </form>
    </AuthFrame>
</template>
