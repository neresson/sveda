<script setup>
import { computed, reactive, ref } from 'vue';
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
useDocumentTitle('runtime.document_title');

const saving = ref(false);
const message = ref('');
const error = ref('');

const fromDocument = (document) => {
    const compaction = document.compaction ?? {};
    const cors = document.cors ?? {};
    const origins = Array.isArray(cors.allowed_origins) ? cors.allowed_origins : [];
    const failover = Array.isArray(document.failover) ? document.failover : [];

    return {
        default_model: String(document.default_model ?? document.model ?? ''),
        failover: failover.join(', '),
        max_steps: Number(document.max_steps ?? 30),
        compaction_enabled: Boolean(compaction.enabled ?? true),
        min_messages: Number(compaction.min_messages ?? 40),
        keep_tail_messages: Number(compaction.keep_tail_messages ?? 20),
        cors_origins: origins.join('\n'),
    };
};

const form = reactive(fromDocument(props.settings));

const modelOptions = computed(() => (Array.isArray(props.settings.models) ? props.settings.models : []));

const hasDefaultInCatalog = computed(() =>
    modelOptions.value.some((model) => model.id === form.default_model),
);

const splitList = (value, pattern) =>
    String(value)
        .split(pattern)
        .map((item) => item.trim())
        .filter(Boolean);

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
                default_model: form.default_model.trim(),
                failover: splitList(form.failover, ','),
                max_steps: Number(form.max_steps),
                compaction: {
                    enabled: Boolean(form.compaction_enabled),
                    min_messages: Number(form.min_messages),
                    keep_tail_messages: Number(form.keep_tail_messages),
                },
                cors: {
                    allowed_origins: splitList(form.cors_origins, /\r?\n/),
                },
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
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('runtime.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('runtime.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">{{ t('runtime.subtitle') }}</p>
        </div>

        <form class="flex flex-col gap-4 border border-ink p-4 lg:p-6" @submit.prevent="save">
            <div class="grid gap-4 md:grid-cols-2">
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('runtime.default_model') }}</span>
                    <select
                        v-if="modelOptions.length"
                        v-model="form.default_model"
                        name="default_model"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                        <option value="">{{ t('runtime.default_model_empty') }}</option>
                        <option v-if="form.default_model && !hasDefaultInCatalog" :value="form.default_model">
                            {{ form.default_model }}
                        </option>
                        <option v-for="model in modelOptions" :key="model.id" :value="model.id">
                            {{ model.label }} ({{ model.id }})
                        </option>
                    </select>
                    <input
                        v-else
                        v-model="form.default_model"
                        name="default_model"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                    <span class="font-mono text-[11px] text-muted">{{ t('runtime.default_model_hint') }}</span>
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('runtime.max_steps') }}</span>
                    <input
                        v-model.number="form.max_steps"
                        name="max_steps"
                        type="number"
                        min="1"
                        max="200"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <label class="flex flex-col gap-2 md:col-span-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('runtime.failover') }}</span>
                    <input
                        v-model="form.failover"
                        name="failover"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                    <span class="font-mono text-[11px] text-muted">{{ t('runtime.failover_hint') }}</span>
                </label>
            </div>

            <div class="grid gap-4 border-t border-grid pt-4 md:grid-cols-2">
                <label class="flex items-center gap-2 font-mono text-xs md:col-span-2">
                    <input v-model="form.compaction_enabled" type="checkbox" class="size-3.5 border border-ink accent-ink">
                    {{ t('runtime.compaction') }} · {{ t('runtime.compaction_enabled') }}
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('runtime.min_messages') }}</span>
                    <input
                        v-model.number="form.min_messages"
                        name="compaction_min_messages"
                        type="number"
                        min="1"
                        max="500"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('runtime.keep_tail') }}</span>
                    <input
                        v-model.number="form.keep_tail_messages"
                        name="compaction_keep_tail"
                        type="number"
                        min="1"
                        max="500"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
            </div>

            <label class="flex flex-col gap-2 border-t border-grid pt-4">
                <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('runtime.cors') }}</span>
                <textarea
                    v-model="form.cors_origins"
                    name="cors_allowed_origins"
                    rows="4"
                    class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                ></textarea>
                <span class="font-mono text-[11px] text-muted">{{ t('runtime.cors_hint') }}</span>
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
