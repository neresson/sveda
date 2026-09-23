<script setup lang="ts">
import { cn } from '../../lib/utils.js';
import type { HTMLAttributes } from 'vue';

const props = defineProps<{ class?: HTMLAttributes['class'] }>();
</script>

<template>
    <div :class="cn('flex flex-col-reverse main-sm:flex-row main-sm:justify-end main-sm:gap-x-2', props.class)">
        <slot />
    </div>
</template>
