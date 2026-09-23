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

const emptyMcp = () => ({ mcpServers: {} });

const fromSettings = (mcp) => {
    if (!mcp || typeof mcp !== 'object' || Array.isArray(mcp)) {
        return emptyMcp();
    }

    const servers = mcp.mcpServers;
    if (Array.isArray(servers) && servers.length === 0) {
        return emptyMcp();
    }

    return {
        mcpServers: servers && typeof servers === 'object' && !Array.isArray(servers) ? servers : {},
    };
};

const stringify = (mcp) => JSON.stringify(fromSettings(mcp), null, 2);

const source = ref(stringify(props.settings.mcp));
const saving = ref(false);
const message = ref('');
const error = ref('');

const { t } = useI18n();
useDocumentTitle('mcp.document_title');

const save = async () => {
    if (saving.value) {
        return;
    }

    let parsed;
    try {
        parsed = JSON.parse(source.value);
    } catch {
        error.value = t('mcp.invalid_json');
        message.value = '';
        return;
    }

    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        error.value = t('mcp.invalid_json');
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
                mcp: parsed,
            }),
        });

        const body = await response.json().catch(() => ({}));

        if (!response.ok) {
            error.value = body.message ?? t('common.save_failed');
            return;
        }

        source.value = stringify(body.mcp);
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
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('mcp.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('mcp.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">{{ t('mcp.subtitle') }}</p>
        </div>

        <form class="flex flex-col gap-4 border border-ink p-4 lg:p-6" @submit.prevent="save">
            <label class="flex flex-col gap-2">
                <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('mcp.json') }}</span>
                <textarea
                    v-model="source"
                    name="mcp_json"
                    rows="22"
                    spellcheck="false"
                    class="min-h-[28rem] border border-ink bg-canvas px-3.5 py-3 font-mono text-sm leading-6 outline-none"
                />
            </label>
            <p class="font-mono text-[11px] text-muted">{{ t('mcp.hint') }}</p>
            <div class="flex justify-end">
                <button
                    type="submit"
                    class="sveda-hover bg-ink px-7 py-3.5 text-sm font-semibold text-canvas hover:bg-ink/80 disabled:opacity-40 disabled:hover:bg-ink"
                    :disabled="saving"
                >
                    {{ t('common.save') }}
                </button>
            </div>
        </form>

        <p v-if="error" class="font-mono text-sm">{{ error }}</p>
        <p v-else-if="message" class="font-mono text-sm text-muted">{{ message }}</p>
    </div>
</template>
