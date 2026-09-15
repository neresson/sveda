<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDocumentTitle } from './useDocumentTitle';

const props = defineProps({
    csrf: { type: String, required: true },
    logoutUrl: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
    usage: { type: Object, default: () => ({}) },
});

const { t, locale } = useI18n();
useDocumentTitle('usage.document_title');

const emptyUsage = () => ({
    by_model: [],
    requests: {
        data: [],
        current_page: 1,
        last_page: 1,
        per_page: 25,
        total: 0,
        prev_page_url: null,
        next_page_url: null,
    },
});

const usage = computed(() => {
    const incoming = props.usage && typeof props.usage === 'object' ? props.usage : {};

    return {
        ...emptyUsage(),
        ...incoming,
        requests: {
            ...emptyUsage().requests,
            ...(incoming.requests && typeof incoming.requests === 'object' ? incoming.requests : {}),
        },
    };
});

const byModel = computed(() => (Array.isArray(usage.value.by_model) ? usage.value.by_model : []));
const rows = computed(() => (Array.isArray(usage.value.requests.data) ? usage.value.requests.data : []));
const hasModels = computed(() => byModel.value.some((row) => Number(row.tokens_used) > 0 || Number(row.requests) > 0));
const hasRows = computed(() => rows.value.length > 0);
const tokenMax = computed(() => Math.max(0, ...byModel.value.map((row) => Number(row.tokens_used) || 0)));

const formatNumber = (value) =>
    new Intl.NumberFormat(locale.value, { maximumFractionDigits: 0 }).format(Number(value) || 0);

const formatDate = (value) => {
    if (!value) {
        return '';
    }

    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
        return String(value);
    }

    return new Intl.DateTimeFormat(locale.value, {
        dateStyle: 'short',
        timeStyle: 'short',
    }).format(date);
};

const modelLabel = (row) => row.model_label || row.label || t('usage.unknown_model');

const barWidth = (value) => {
    const amount = Number(value) || 0;
    if (tokenMax.value <= 0 || amount <= 0) {
        return '2px';
    }

    return `${Math.max(6, Math.round((amount / tokenMax.value) * 100))}%`;
};
</script>

<template>
    <div class="contents">
        <div>
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('usage.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('usage.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">{{ t('usage.subtitle') }}</p>
        </div>

        <section class="flex flex-col gap-4 border border-ink p-4 lg:p-5">
            <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('usage.models_chart') }}</p>
            <p v-if="!hasModels" class="font-serif text-base text-muted">{{ t('usage.empty_models') }}</p>
            <div v-else class="flex flex-col gap-4">
                <div v-for="row in byModel" :key="row.model || 'unknown'" class="flex flex-col gap-2">
                    <div class="flex items-baseline justify-between gap-4">
                        <p class="min-w-0 truncate font-mono text-[12px] tracking-[0.08em]">{{ modelLabel(row) }}</p>
                        <p class="shrink-0 font-mono text-[11px] tracking-[0.08em] text-muted">
                            {{ formatNumber(row.tokens_used) }} {{ t('usage.tokens_unit', Number(row.tokens_used) || 0) }}
                            ·
                            {{ formatNumber(row.requests) }} {{ t('usage.requests_unit', Number(row.requests) || 0) }}
                        </p>
                    </div>
                    <div class="h-3 w-full bg-grid">
                        <div
                            class="h-full bg-ink veda-bar-x"
                            :style="{ width: barWidth(row.tokens_used) }"
                        ></div>
                    </div>
                </div>
            </div>
        </section>

        <section class="flex flex-col gap-4 border border-ink">
            <div class="flex items-center justify-between gap-3 border-b border-ink px-4 py-4 lg:px-5">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('usage.table_title') }}</p>
                <p class="font-mono text-[11px] tracking-[0.08em] text-muted">
                    {{ t('usage.table_total', { count: formatNumber(usage.requests.total) }) }}
                </p>
            </div>
            <p v-if="!hasRows" class="px-4 pb-4 font-serif text-base text-muted lg:px-5">{{ t('usage.empty_requests') }}</p>
            <div v-else class="overflow-x-auto">
                <table class="w-full border-collapse text-left">
                    <thead>
                        <tr class="border-b border-grid font-mono text-[11px] tracking-[0.14em] text-muted">
                            <th class="px-4 py-3 font-normal lg:px-5">{{ t('usage.column_model') }}</th>
                            <th class="px-4 py-3 font-normal lg:px-5">{{ t('usage.column_date') }}</th>
                            <th class="px-4 py-3 text-right font-normal lg:px-5">{{ t('usage.column_tokens') }}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr
                            v-for="row in rows"
                            :key="row.id"
                            class="border-b border-grid last:border-b-0"
                        >
                            <td class="px-4 py-3 font-mono text-[13px] lg:px-5">{{ modelLabel(row) }}</td>
                            <td class="px-4 py-3 font-mono text-[13px] text-muted lg:px-5">{{ formatDate(row.created_at) }}</td>
                            <td class="px-4 py-3 text-right font-mono text-[13px] lg:px-5">{{ formatNumber(row.tokens_used) }}</td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <div
                v-if="usage.requests.last_page > 1"
                class="flex items-center justify-between gap-3 border-t border-ink px-4 py-3 lg:px-5"
            >
                <a
                    v-if="usage.requests.prev_page_url"
                    :href="usage.requests.prev_page_url"
                    class="veda-hover font-mono text-[11px] tracking-[0.14em] text-ink no-underline hover:text-muted"
                >
                    {{ t('common.prev') }}
                </a>
                <span v-else class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('common.prev') }}</span>
                <p class="font-mono text-[11px] tracking-[0.08em] text-muted">
                    {{ t('usage.page', { current: usage.requests.current_page, last: usage.requests.last_page }) }}
                </p>
                <a
                    v-if="usage.requests.next_page_url"
                    :href="usage.requests.next_page_url"
                    class="veda-hover font-mono text-[11px] tracking-[0.14em] text-ink no-underline hover:text-muted"
                >
                    {{ t('common.next') }}
                </a>
                <span v-else class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('common.next') }}</span>
            </div>
        </section>
    </div>
</template>
