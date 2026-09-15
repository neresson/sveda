<script setup>
import { reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDocumentTitle } from './useDocumentTitle';

const props = defineProps({
    csrf: { type: String, required: true },
    saveUrl: { type: String, required: true },
    logoutUrl: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
    settings: { type: Object, default: () => ({}) },
});

const { t } = useI18n();
useDocumentTitle('prompts.document_title');

const saving = ref(false);
const message = ref('');
const error = ref('');

const fromDocument = (document) => ({
    welcome_message: String(document.welcome_message ?? ''),
    system_prompt: String(document.system_prompt ?? ''),
});

const form = reactive(fromDocument(props.settings));

const save = async () => {
    if (saving.value) {
        return;
    }

    saving.value = true;
    error.value = '';
    message.value = '';

    try {
        const response = await fetch(props.saveUrl, {
            method: 'POST',
            credentials: 'same-origin',
            headers: {
                Accept: 'application/json',
                'Content-Type': 'application/json',
                'X-CSRF-TOKEN': props.csrf,
            },
            body: JSON.stringify({
                welcome_message: form.welcome_message,
                system_prompt: form.system_prompt,
            }),
        });

        const body = await response.json().catch(() => ({}));

        if (!response.ok) {
            error.value = body.message ?? t('common.save_failed');
            return;
        }

        Object.assign(form, fromDocument(body));
        message.value = t('common.saved');
    } catch {
        error.value = t('common.save_failed');
    } finally {
        saving.value = false;
    }
};
</script>

<template>
    <div class="contents">
        <div>
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('prompts.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('prompts.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">{{ t('prompts.subtitle') }}</p>
        </div>

        <form class="flex flex-col gap-4 border border-ink p-4 lg:p-6" @submit.prevent="save">
            <label class="flex flex-col gap-2">
                <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('prompts.welcome') }}</span>
                <textarea
                    v-model="form.welcome_message"
                    name="welcome_message"
                    rows="4"
                    class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                ></textarea>
                <span class="font-mono text-[11px] text-muted">{{ t('prompts.welcome_hint') }}</span>
            </label>

            <label class="flex flex-col gap-2">
                <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('prompts.system') }}</span>
                <textarea
                    v-model="form.system_prompt"
                    name="system_prompt"
                    rows="12"
                    class="min-h-48 border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                ></textarea>
                <span class="font-mono text-[11px] text-muted">{{ t('prompts.system_hint') }}</span>
            </label>

            <div class="flex items-center gap-4">
                <button
                    type="submit"
                    class="veda-hover bg-ink px-7 py-3.5 text-sm font-semibold text-canvas hover:bg-ink/80 disabled:opacity-40 disabled:hover:bg-ink"
                    :disabled="saving"
                >
                    {{ t('common.save') }}
                </button>
                <p v-if="error" class="font-mono text-sm">{{ error }}</p>
                <p v-else-if="message" class="font-mono text-sm text-muted">{{ message }}</p>
            </div>
        </form>
    </div>
</template>
