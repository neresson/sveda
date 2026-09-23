<script setup lang="ts">
import { cn } from '../../lib/utils.js';
import type { HTMLAttributes } from 'vue';

const props = defineProps<{ class?: HTMLAttributes['class'] }>();
</script>

<template>
    <div :class="cn('flex flex-col gap-y-2 text-center main-sm:text-left', props.class)">
        <slot />
    </div>
</template>
