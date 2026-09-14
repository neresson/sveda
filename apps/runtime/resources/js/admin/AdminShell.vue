<script setup>
import { onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import AdminNav from './AdminNav.vue';
import LocaleSwitch from './LocaleSwitch.vue';

defineProps({
    csrf: { type: String, required: true },
    logoutUrl: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
    current: { type: String, default: 'runtime' },
});

const { t } = useI18n();

const MOBILE_QUERY = '(max-width: 1023px)';

const navOpen = ref(false);
let media = null;

const closeNav = () => {
    navOpen.value = false;
};

const toggleNav = () => {
    navOpen.value = !navOpen.value;
};

const onMediaChange = (event) => {
    if (!event.matches) {
        navOpen.value = false;
    }
};

const onKeydown = (event) => {
    if (event.key === 'Escape') {
        closeNav();
    }
};

onMounted(() => {
    media = window.matchMedia(MOBILE_QUERY);
    media.addEventListener('change', onMediaChange);
    window.addEventListener('keydown', onKeydown);
});

onUnmounted(() => {
    media?.removeEventListener('change', onMediaChange);
    window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
    <div class="relative flex h-dvh w-full flex-col overflow-hidden border border-ink bg-canvas">
        <header class="flex items-center gap-3 border-b border-ink px-4 py-3 lg:gap-4 lg:px-8 lg:py-4">
            <button
                type="button"
                class="flex size-8 shrink-0 flex-col items-center justify-center gap-1 border border-ink lg:hidden"
                :aria-expanded="navOpen"
                aria-controls="veda-admin-nav"
                :aria-label="navOpen ? t('common.close_menu') : t('common.open_menu')"
                @click="toggleNav"
            >
                <span class="block h-px w-3.5 bg-ink"></span>
                <span class="block h-px w-3.5 bg-ink"></span>
                <span class="block h-px w-3.5 bg-ink"></span>
            </button>
            <span class="hidden size-5 items-center justify-center border border-ink lg:flex">
                <span class="size-2 bg-ink"></span>
            </span>
            <p class="font-mono text-xs tracking-[0.16em]">{{ t('shell.brand') }}</p>
            <LocaleSwitch class="ml-auto" />
            <form :action="logoutUrl" method="post">
                <input type="hidden" name="_token" :value="csrf">
                <button type="submit" class="font-mono text-[11px] tracking-[0.16em]">{{ t('common.logout') }}</button>
            </form>
        </header>

        <div class="relative flex min-h-0 flex-1">
            <aside class="hidden w-[260px] shrink-0 flex-col border-r border-grid p-6 lg:flex">
                <AdminNav :current="current" :urls="urls" />
            </aside>

            <div
                v-if="navOpen"
                class="absolute inset-0 z-40 lg:hidden"
            >
                <button
                    type="button"
                    class="absolute inset-0 bg-ink/25"
                    :aria-label="t('common.close_menu')"
                    @click="closeNav"
                ></button>
                <aside
                    id="veda-admin-nav"
                    class="relative flex h-full w-[min(100%,20rem)] flex-col border-r border-ink bg-canvas p-5"
                >
                    <div class="mb-5 flex items-center justify-between">
                        <p class="font-mono text-xs tracking-[0.16em]">{{ t('common.menu') }}</p>
                        <button
                            type="button"
                            class="font-mono text-[11px] tracking-[0.16em]"
                            @click="closeNav"
                        >
                            {{ t('common.close') }}
                        </button>
                    </div>
                    <AdminNav :current="current" :urls="urls" />
                </aside>
            </div>

            <main class="flex min-w-0 flex-1 flex-col gap-5 overflow-y-auto px-4 py-6 sm:px-6 lg:gap-6 lg:px-12 lg:py-10">
                <slot />
            </main>
        </div>
    </div>
</template>
