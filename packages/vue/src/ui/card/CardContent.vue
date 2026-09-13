<script setup lang="ts">
import { cn } from '../../lib/utils.js';
import type { HTMLAttributes } from 'vue';

const props = defineProps<{
    class?: HTMLAttributes['class'];
}>();
</script>

<template>
    <div :class="cn('p-6 pt-0 main-max-md:p-4', props.class)" >
        <slot />
    </div>
</template>
