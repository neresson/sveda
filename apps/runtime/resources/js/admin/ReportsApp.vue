<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDocumentTitle } from './useDocumentTitle';

const props = defineProps({
    csrf: { type: String, required: true },
    logoutUrl: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
    reports: { type: Array, default: () => [] },
});

const { t, locale } = useI18n();
useDocumentTitle('reports.document_title');

const rows = computed(() => (Array.isArray(props.reports) ? props.reports : []));

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

const reasonLabel = (reason) => {
    const key = `reports.reason_${reason}`;
    const label = t(key);
    return label === key ? reason : label;
};
</script>

<template>
    <div class="contents">
        <div>
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('reports.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('reports.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">{{ t('reports.subtitle') }}</p>
        </div>

        <section class="flex flex-col gap-4 border border-ink">
            <div class="flex items-center justify-between gap-3 border-b border-ink px-4 py-4 lg:px-5">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('reports.table_title') }}</p>
                <p class="font-mono text-[11px] tracking-[0.08em] text-muted">{{ rows.length }}</p>
            </div>
            <p v-if="rows.length === 0" class="px-4 pb-4 font-serif text-base text-muted lg:px-5">{{ t('reports.empty') }}</p>
            <div v-else class="flex flex-col">
                <article
                    v-for="row in rows"
                    :key="row.id"
                    class="flex flex-col gap-2 border-b border-grid px-4 py-4 last:border-b-0 lg:px-5"
                >
                    <div class="flex flex-wrap items-baseline justify-between gap-3">
                        <p class="font-mono text-[12px] tracking-[0.08em]">{{ reasonLabel(row.reason) }}</p>
                        <p class="font-mono text-[11px] tracking-[0.08em] text-muted">{{ formatDate(row.created_at) }}</p>
                    </div>
                    <p class="whitespace-pre-wrap font-serif text-base text-ink">{{ row.excerpt || t('reports.no_excerpt') }}</p>
                    <p class="font-mono text-[11px] tracking-[0.08em] text-muted">{{ row.visitor_id }}</p>
                </article>
            </div>
        </section>
    </div>
</template>
