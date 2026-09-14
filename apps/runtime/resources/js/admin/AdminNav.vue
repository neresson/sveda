<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

const props = defineProps({
    current: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
});

const { t } = useI18n();

const items = [
    { id: 'runtime' },
    { id: 'models' },
    { id: 'prompts' },
];

const currentItem = computed(
    () => items.find((item) => item.id === props.current) ?? items[0],
);

const hrefFor = (id) => {
    const url = props.urls?.[id];

    return typeof url === 'string' && url !== '' ? url : '';
};

const itemClass = (id) =>
    id === props.current
        ? 'bg-ink px-4 py-3 font-mono text-[11px] tracking-[0.14em] text-canvas no-underline'
        : 'border border-grid px-4 py-3 font-mono text-[11px] tracking-[0.14em] text-ink no-underline';
</script>

<template>
    <nav class="flex min-h-0 flex-1 flex-col gap-2">
        <template v-for="item in items" :key="item.id">
            <a
                v-if="hrefFor(item.id)"
                :href="hrefFor(item.id)"
                :class="itemClass(item.id)"
            >
                {{ t(`nav.${item.id}`) }}
            </a>
            <span
                v-else
                :class="itemClass(item.id)"
            >
                {{ t(`nav.${item.id}`) }}
            </span>
        </template>
        <p class="mt-auto font-mono text-[11px] tracking-[0.12em] text-muted">
            {{ t(`nav.${currentItem.id}_index`) }}  /  {{ t(`nav.${currentItem.id}`) }}
        </p>
    </nav>
</template>
