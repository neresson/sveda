<script setup lang="ts">
import { cn } from '../../lib/utils.js';
import type { HTMLAttributes } from 'vue';

const props = defineProps<{
    class?: HTMLAttributes['class'];
}>();
</script>

<template>
    <h3 :class="cn('text-2xl font-bold leading-none tracking-tight', props.class)">
        <slot />
    </h3>
</template>
