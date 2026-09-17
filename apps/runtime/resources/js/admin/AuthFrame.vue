<script setup>
import LocaleSwitch from './LocaleSwitch.vue';

defineProps({
    index: { type: String, required: true },
    brand: { type: String, required: true },
    footer: { type: String, default: '' },
    variant: { type: String, default: 'login' },
});
</script>

<template>
    <main class="relative flex h-dvh overflow-hidden border border-ink max-lg:flex-col">
        <div class="sveda-grid pointer-events-none absolute inset-0"></div>
        <div
            v-if="variant === 'login'"
            class="pointer-events-none absolute -bottom-16 -left-16 size-[280px] rotate-45 border border-ink max-md:hidden"
        ></div>
        <div
            v-if="variant === 'setup'"
            class="pointer-events-none absolute inset-y-0 right-0 w-18 bg-ink max-lg:hidden"
        ></div>
        <p
            class="absolute font-mono text-xs tracking-[0.2em] text-muted"
            :class="variant === 'setup'
                ? 'left-12 top-12 max-lg:left-5 max-lg:top-5'
                : 'right-12 top-12 max-lg:right-5 max-lg:top-5'"
        >
            {{ index }}
        </p>

        <section class="relative flex flex-1 flex-col justify-between px-16 py-16 max-lg:flex-none max-lg:justify-start max-lg:gap-6 max-lg:px-5 max-lg:py-8">
            <div class="flex items-center gap-3">
                <span class="flex size-7 items-center justify-center border border-ink">
                    <span class="size-3 bg-ink"></span>
                </span>
                <p class="font-mono text-xs tracking-[0.16em]">{{ brand }}</p>
                <LocaleSwitch />
            </div>

            <div class="max-w-xl">
                <slot name="title" />
            </div>

            <p v-if="footer" class="font-mono text-[11px] tracking-[0.14em] text-muted max-lg:hidden">
                {{ footer }}
            </p>
        </section>

        <section
            class="relative flex w-full max-w-[560px] flex-col justify-center border-l border-grid px-16 py-16 max-lg:max-w-none max-lg:flex-1 max-lg:justify-start max-lg:overflow-y-auto max-lg:border-l-0 max-lg:border-t max-lg:px-5 max-lg:py-8"
            :class="variant === 'setup' ? 'z-10 pr-24 max-lg:pr-5' : ''"
        >
            <slot />
        </section>
    </main>
</template>
