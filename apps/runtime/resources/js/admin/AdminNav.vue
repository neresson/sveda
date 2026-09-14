<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

const props = defineProps({
    current: { type: String, required: true },
});

const { t } = useI18n();

const items = [
    { id: 'runtime' },
    { id: 'models' },
    { id: 'prompts' },
    { id: 'access' },
];

const currentItem = computed(
    () => items.find((item) => item.id === props.current) ?? items[0],
);
</script>

<template>
    <nav class="flex min-h-0 flex-1 flex-col gap-2">
        <span
            v-for="item in items"
            :key="item.id"
            :class="item.id === current
                ? 'bg-ink px-4 py-3 font-mono text-[11px] tracking-[0.14em] text-canvas'
                : 'border border-grid px-4 py-3 font-mono text-[11px] tracking-[0.14em] text-ink'"
        >
            {{ t(`nav.${item.id}`) }}
        </span>
        <p class="mt-auto font-mono text-[11px] tracking-[0.12em] text-muted">
            {{ t(`nav.${currentItem.id}_index`) }}  /  {{ t(`nav.${currentItem.id}`) }}
        </p>
    </nav>
</template>
