<script setup>
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDocumentTitle } from './useDocumentTitle';

const props = defineProps({
    csrf: { type: String, required: true },
    saveUrl: { type: String, required: true },
    logoutUrl: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
    settings: { type: Object, default: () => ({}) },
});

const fromSettings = (policies) => {
    if (!policies || typeof policies !== 'object' || Array.isArray(policies)) {
        return {};
    }

    return policies;
};

const stringify = (policies) => JSON.stringify(fromSettings(policies), null, 2);

const source = ref(stringify(props.settings.policies));
const saving = ref(false);
const message = ref('');
const error = ref('');

const { t } = useI18n();
useDocumentTitle('policies.document_title');

const save = async () => {
    if (saving.value) {
        return;
    }

    let parsed;
    try {
        parsed = JSON.parse(source.value);
    } catch {
        error.value = t('policies.invalid_json');
        message.value = '';
        return;
    }

    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        error.value = t('policies.invalid_json');
        message.value = '';
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
                policies: parsed,
            }),
        });

        const body = await response.json().catch(() => ({}));

        if (!response.ok) {
            error.value = body.message ?? t('common.save_failed');
            return;
        }

        source.value = stringify(body.policies);
        message.value = t('common.saved');
    } catch {
        error.value = t('common.save_failed');
    } finally {
        saving.value = false;
    }
};
</script>

<template>
    <section class="space-y-6">
        <header>
            <h1 class="font-mono text-xl tracking-[0.12em] text-ink">
                {{ t('policies.title') }}
            </h1>
            <p class="mt-2 max-w-3xl text-sm text-muted">
                {{ t('policies.lede') }}
            </p>
        </header>

        <textarea
            v-model="source"
            class="min-h-[28rem] w-full border border-grid bg-canvas p-4 font-mono text-xs text-ink"
            spellcheck="false"
        />

        <div class="flex items-center gap-4">
            <button
                type="button"
                class="sveda-hover border border-grid px-4 py-2 font-mono text-[11px] tracking-[0.14em] text-ink"
                :disabled="saving"
                @click="save"
            >
                {{ saving ? t('common.saving') : t('common.save') }}
            </button>
            <p v-if="message" class="font-mono text-[11px] text-ink">{{ message }}</p>
            <p v-if="error" class="font-mono text-[11px] text-danger">{{ error }}</p>
        </div>
    </section>
</template>
