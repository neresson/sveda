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
useDocumentTitle('security.document_title');

const saving = ref(false);
const message = ref('');
const error = ref('');

const minutesFromSecs = (secs) => {
    const value = Number(secs ?? 60);
    if (!Number.isFinite(value) || value <= 0) {
        return 1;
    }
    return Math.max(1, Math.round(value / 60));
};

const fromDocument = (document) => {
    const cors = document.cors ?? {};
    const origins = Array.isArray(cors.allowed_origins) ? cors.allowed_origins : [];
    const security = document.security ?? {};

    return {
        cors_origins: origins.join('\n'),
        occupancy_global: Number(security.occupancy_global ?? 0),
        occupancy_per_visitor: Number(security.occupancy_per_visitor ?? 0),
        embed_token_throttle_max: Number(security.embed_token_throttle_max ?? 0),
        embed_token_throttle_minutes: minutesFromSecs(security.embed_token_throttle_window_secs),
        stream_throttle_max: Number(security.stream_throttle_max ?? 0),
        stream_throttle_minutes: minutesFromSecs(security.stream_throttle_window_secs),
        client_ip_header: String(security.client_ip_header ?? ''),
        ip_throttle_max: Number(security.ip_throttle_max ?? 0),
        ip_throttle_minutes: minutesFromSecs(security.ip_throttle_window_secs),
    };
};

const form = reactive(fromDocument(props.settings));

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
                cors: {
                    allowed_origins: splitList(form.cors_origins, /\r?\n/),
                },
                security: {
                    occupancy_global: Number(form.occupancy_global),
                    occupancy_per_visitor: Number(form.occupancy_per_visitor),
                    embed_token_throttle_max: Number(form.embed_token_throttle_max),
                    embed_token_throttle_window_secs: Number(form.embed_token_throttle_minutes) * 60,
                    stream_throttle_max: Number(form.stream_throttle_max),
                    stream_throttle_window_secs: Number(form.stream_throttle_minutes) * 60,
                    client_ip_header: form.client_ip_header.trim(),
                    ip_throttle_max: Number(form.ip_throttle_max),
                    ip_throttle_window_secs: Number(form.ip_throttle_minutes) * 60,
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
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('security.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('security.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">{{ t('security.subtitle') }}</p>
        </div>

        <form class="flex flex-col gap-4 border border-ink p-4 lg:p-6" @submit.prevent="save">
            <label class="flex flex-col gap-2">
                <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.cors') }}</span>
                <textarea
                    v-model="form.cors_origins"
                    name="cors_allowed_origins"
                    rows="4"
                    class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                ></textarea>
                <span class="font-mono text-[11px] text-muted">{{ t('security.cors_hint') }}</span>
            </label>

            <div class="grid gap-4 border-t border-grid pt-4 md:grid-cols-2">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted md:col-span-2">{{ t('security.occupancy') }}</p>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.occupancy_global') }}</span>
                    <input
                        v-model.number="form.occupancy_global"
                        name="occupancy_global"
                        type="number"
                        min="0"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.occupancy_per_visitor') }}</span>
                    <input
                        v-model.number="form.occupancy_per_visitor"
                        name="occupancy_per_visitor"
                        type="number"
                        min="0"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <span class="font-mono text-[11px] text-muted md:col-span-2">{{ t('security.occupancy_hint') }}</span>
            </div>

            <div class="grid gap-4 border-t border-grid pt-4 md:grid-cols-2">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted md:col-span-2">{{ t('security.embed_throttle') }}</p>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.throttle_max') }}</span>
                    <input
                        v-model.number="form.embed_token_throttle_max"
                        name="embed_token_throttle_max"
                        type="number"
                        min="0"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.throttle_minutes') }}</span>
                    <input
                        v-model.number="form.embed_token_throttle_minutes"
                        name="embed_token_throttle_minutes"
                        type="number"
                        min="1"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <span class="font-mono text-[11px] text-muted md:col-span-2">{{ t('security.embed_throttle_hint') }}</span>
            </div>

            <div class="grid gap-4 border-t border-grid pt-4 md:grid-cols-2">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted md:col-span-2">{{ t('security.stream_throttle') }}</p>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.throttle_max') }}</span>
                    <input
                        v-model.number="form.stream_throttle_max"
                        name="stream_throttle_max"
                        type="number"
                        min="0"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.throttle_minutes') }}</span>
                    <input
                        v-model.number="form.stream_throttle_minutes"
                        name="stream_throttle_minutes"
                        type="number"
                        min="1"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <span class="font-mono text-[11px] text-muted md:col-span-2">{{ t('security.stream_throttle_hint') }}</span>
            </div>

            <div class="grid gap-4 border-t border-grid pt-4 md:grid-cols-2">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted md:col-span-2">{{ t('security.ip_throttle') }}</p>
                <label class="flex flex-col gap-2 md:col-span-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.client_ip_header') }}</span>
                    <input
                        v-model="form.client_ip_header"
                        name="client_ip_header"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                        placeholder="CF-Connecting-IP"
                    >
                    <span class="font-mono text-[11px] text-muted">{{ t('security.client_ip_header_hint') }}</span>
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.throttle_max') }}</span>
                    <input
                        v-model.number="form.ip_throttle_max"
                        name="ip_throttle_max"
                        type="number"
                        min="0"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('security.throttle_minutes') }}</span>
                    <input
                        v-model.number="form.ip_throttle_minutes"
                        name="ip_throttle_minutes"
                        type="number"
                        min="1"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                </label>
                <span class="font-mono text-[11px] text-muted md:col-span-2">{{ t('security.ip_throttle_hint') }}</span>
            </div>

            <div class="flex items-center gap-4">
                <button
                    type="submit"
                    class="sveda-hover bg-ink px-7 py-3.5 text-sm font-semibold text-canvas hover:bg-ink/80 disabled:opacity-40 disabled:hover:bg-ink"
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
