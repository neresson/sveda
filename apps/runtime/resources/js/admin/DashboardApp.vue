<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDocumentTitle } from './useDocumentTitle';

const props = defineProps({
    csrf: { type: String, required: true },
    logoutUrl: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
    stats: { type: Object, default: () => ({}) },
});

const { t, locale } = useI18n();
useDocumentTitle('dashboard.document_title');

const emptyStats = () => ({
    period_days: 14,
    requests: 0,
    completed: 0,
    failed: 0,
    pending: 0,
    prompt_tokens: 0,
    completion_tokens: 0,
    tokens_used: 0,
    unsplit_tokens: 0,
    users: 0,
    conversations: 0,
    avg_tokens: 0,
    success_rate: 0,
    series: [],
});

const stats = computed(() => ({
    ...emptyStats(),
    ...(props.stats && typeof props.stats === 'object' ? props.stats : {}),
}));

const series = computed(() => (Array.isArray(stats.value.series) ? stats.value.series : []));
const hasTraffic = computed(() => Number(stats.value.requests) > 0);
const hasTokenSplit = computed(
    () => Number(stats.value.prompt_tokens) > 0 || Number(stats.value.completion_tokens) > 0,
);
const hasUnsplit = computed(() => Number(stats.value.unsplit_tokens) > 0);

const formatNumber = (value) =>
    new Intl.NumberFormat(locale.value, { maximumFractionDigits: 0 }).format(Number(value) || 0);

const barHeight = (value, max) => {
    const amount = Number(value) || 0;
    if (max <= 0 || amount <= 0) {
        return '2px';
    }

    return `${Math.max(6, Math.round((amount / max) * 100))}%`;
};

const stackHeight = (value, max) => {
    const amount = Number(value) || 0;
    if (max <= 0 || amount <= 0) {
        return '0px';
    }

    return `${Math.max(6, Math.round((amount / max) * 100))}%`;
};

const requestMax = computed(() => Math.max(0, ...series.value.map((day) => Number(day.requests) || 0)));

const tokenMax = computed(() =>
    Math.max(0, ...series.value.map((day) => Number(day.tokens_used) || 0)),
);

const dayLabel = (date) => {
    const parts = String(date).split('-');

    return parts.length === 3 ? String(Number(parts[2])) : date;
};

const cards = computed(() => [
    { key: 'requests', value: stats.value.requests },
    { key: 'prompt_tokens', value: stats.value.prompt_tokens },
    { key: 'completion_tokens', value: stats.value.completion_tokens },
    { key: 'tokens_used', value: stats.value.tokens_used },
]);

const extras = computed(() => [
    { key: 'conversations', value: stats.value.conversations },
    { key: 'users', value: stats.value.users },
    { key: 'avg_tokens', value: stats.value.avg_tokens },
    { key: 'failed', value: stats.value.failed },
    { key: 'pending', value: stats.value.pending },
    { key: 'success_rate', value: `${formatNumber(stats.value.success_rate)}%`, raw: true },
]);
</script>

<template>
    <div class="contents">
        <div>
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('dashboard.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('dashboard.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">
                {{ t('dashboard.subtitle', { days: stats.period_days }) }}
            </p>
        </div>

        <div class="grid grid-cols-2 gap-px border border-ink bg-ink lg:grid-cols-4">
            <div
                v-for="card in cards"
                :key="card.key"
                class="flex flex-col gap-2 bg-canvas p-4 lg:p-5"
            >
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t(`dashboard.${card.key}`) }}</p>
                <p class="text-3xl font-semibold tracking-tight lg:text-4xl">{{ formatNumber(card.value) }}</p>
            </div>
        </div>

        <p v-if="hasUnsplit" class="font-serif text-base text-muted">
            {{ t('dashboard.unsplit_hint', { count: formatNumber(stats.unsplit_tokens) }) }}
        </p>

        <p v-if="!hasTraffic" class="font-serif text-base text-muted">{{ t('dashboard.empty') }}</p>

        <div class="grid gap-4 lg:grid-cols-2">
            <section class="flex flex-col gap-4 border border-ink p-4 lg:p-5">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('dashboard.requests_chart') }}</p>
                <div class="flex h-36 items-end gap-1">
                    <div
                        v-for="day in series"
                        :key="`req-${day.date}`"
                        class="flex h-full min-w-0 flex-1 items-end"
                        :title="`${day.date}: ${formatNumber(day.requests)}`"
                    >
                        <div
                            class="w-full veda-bar-y"
                            :class="Number(day.requests) > 0 ? 'bg-ink' : 'bg-grid'"
                            :style="{ height: barHeight(day.requests, requestMax) }"
                        ></div>
                    </div>
                </div>
                <div class="flex justify-between font-mono text-[10px] tracking-[0.08em] text-muted">
                    <span>{{ series[0] ? dayLabel(series[0].date) : '' }}</span>
                    <span>{{ series[series.length - 1] ? dayLabel(series[series.length - 1].date) : '' }}</span>
                </div>
            </section>

            <section class="flex flex-col gap-4 border border-ink p-4 lg:p-5">
                <div class="flex items-center justify-between gap-3">
                    <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('dashboard.tokens_chart') }}</p>
                    <p v-if="hasTokenSplit && hasUnsplit" class="font-mono text-[10px] tracking-[0.08em] text-muted">
                        <span class="mr-2 inline-block size-2 bg-ink"></span>{{ t('dashboard.input') }}
                        <span class="ml-3 mr-2 inline-block size-2 bg-ink/35"></span>{{ t('dashboard.output') }}
                        <span class="ml-3 mr-2 inline-block size-2 bg-ink/15"></span>{{ t('dashboard.unsplit') }}
                    </p>
                    <p v-else-if="hasTokenSplit" class="font-mono text-[10px] tracking-[0.08em] text-muted">
                        <span class="mr-2 inline-block size-2 bg-ink"></span>{{ t('dashboard.input') }}
                        <span class="ml-3 mr-2 inline-block size-2 bg-ink/35"></span>{{ t('dashboard.output') }}
                    </p>
                    <p v-else class="font-mono text-[10px] tracking-[0.08em] text-muted">{{ t('dashboard.total') }}</p>
                </div>
                <div class="flex h-36 items-end gap-1">
                    <div
                        v-for="day in series"
                        :key="`tok-${day.date}`"
                        class="flex h-full min-w-0 flex-1 items-end justify-center gap-px"
                        :title="`${day.date}: ${formatNumber(day.tokens_used)}`"
                    >
                        <div class="veda-bar-y flex h-full w-full flex-col justify-end">
                            <div
                                v-if="hasUnsplit"
                                class="w-full"
                                :class="Number(day.unsplit_tokens) > 0 ? 'bg-ink/15' : ''"
                                :style="{ height: stackHeight(day.unsplit_tokens, tokenMax) }"
                            ></div>
                            <div
                                class="w-full"
                                :class="Number(day.prompt_tokens) > 0 ? 'bg-ink' : (Number(day.tokens_used) > 0 && !hasTokenSplit ? 'bg-ink' : '')"
                                :style="{ height: stackHeight(hasTokenSplit ? day.prompt_tokens : (hasUnsplit ? 0 : day.tokens_used), tokenMax) }"
                            ></div>
                            <div
                                v-if="hasTokenSplit"
                                class="w-full"
                                :class="Number(day.completion_tokens) > 0 ? 'bg-ink/35' : ''"
                                :style="{ height: stackHeight(day.completion_tokens, tokenMax) }"
                            ></div>
                        </div>
                    </div>
                </div>
                <div class="flex justify-between font-mono text-[10px] tracking-[0.08em] text-muted">
                    <span>{{ series[0] ? dayLabel(series[0].date) : '' }}</span>
                    <span>{{ series[series.length - 1] ? dayLabel(series[series.length - 1].date) : '' }}</span>
                </div>
            </section>
        </div>

        <div class="grid grid-cols-2 gap-px border border-grid bg-grid sm:grid-cols-3 lg:grid-cols-6">
            <div
                v-for="item in extras"
                :key="item.key"
                class="flex flex-col gap-1 bg-canvas p-4"
            >
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t(`dashboard.${item.key}`) }}</p>
                <p class="font-mono text-lg">{{ item.raw ? item.value : formatNumber(item.value) }}</p>
            </div>
        </div>
    </div>
</template>
