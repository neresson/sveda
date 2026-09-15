<script setup>
import { computed, onMounted, onUnmounted, provide, reactive, ref, watch } from 'vue';
import { applyVedaAppearance } from '@veda-ai/vue';
import AdminShell from './AdminShell.vue';
import AdminVedaChat from './AdminVedaChat.vue';
import { shouldInterceptAdminClick } from './adminSpa';

const props = defineProps({
    initial: { type: Object, required: true },
    pages: { type: Object, required: true },
    enableChat: { type: Boolean, default: false },
});

const payload = reactive({ ...props.initial });
const page = ref(payload.page && props.pages[payload.page] ? payload.page : 'dashboard');
const href = ref(typeof window === 'undefined' ? '' : window.location.href);
const visiting = ref(false);

provide('adminHref', href);

const pageProps = computed(() => {
    const propsForPage = {
        csrf: payload.csrf,
        saveUrl: payload.saveUrl,
        logoutUrl: payload.logoutUrl,
        urls: payload.urls,
        settings: payload.settings,
        stats: payload.stats,
        usage: payload.usage,
        codeIndex: payload.codeIndex,
    };

    if (page.value === 'appearance') {
        return {
            ...propsForPage,
            appearancePresets: payload.appearancePresets,
        };
    }

    return propsForPage;
});

const visit = async (nextHref, { historyMode = 'push' } = {}) => {
    const url = new URL(nextHref, window.location.origin);
    if (visiting.value) {
        return;
    }

    visiting.value = true;

    try {
        const response = await fetch(url.toString(), {
            credentials: 'same-origin',
            headers: {
                Accept: 'application/json',
                'X-Requested-With': 'XMLHttpRequest',
                'X-CSRF-TOKEN': String(payload.csrf ?? ''),
            },
        });

        const contentType = response.headers.get('content-type') ?? '';
        if (response.status === 401 || !contentType.includes('application/json')) {
            window.location.assign(url.toString());
            return;
        }

        if (!response.ok) {
            window.location.assign(url.toString());
            return;
        }

        const next = await response.json();
        if (!next?.page || !props.pages[next.page]) {
            window.location.assign(url.toString());
            return;
        }

        Object.assign(payload, next);
        page.value = next.page;
        href.value = url.toString();

        if (historyMode === 'push') {
            history.pushState({ vedaAdmin: true }, '', url);
        } else if (historyMode === 'replace') {
            history.replaceState({ vedaAdmin: true }, '', url);
        }
    } catch {
        window.location.assign(url.toString());
    } finally {
        visiting.value = false;
    }
};

const onDocumentClick = (event) => {
    if (!shouldInterceptAdminClick(event, payload.urls)) {
        return;
    }

    event.preventDefault();
    const link = event.target instanceof Element ? event.target.closest('a[href]') : null;
    if (!link) {
        return;
    }

    const url = new URL(link.href, window.location.origin);
    if (url.href === window.location.href) {
        return;
    }

    void visit(url.href);
};

const onPopState = () => {
    href.value = window.location.href;
    void visit(window.location.href, { historyMode: 'none' });
};

onMounted(() => {
    history.replaceState({ vedaAdmin: true }, '', window.location.href);
    document.addEventListener('click', onDocumentClick);
    window.addEventListener('popstate', onPopState);
});

watch(
    () => payload.settings?.appearance,
    (value) => {
        applyVedaAppearance(value);
    },
    { immediate: true, deep: true },
);

onUnmounted(() => {
    document.removeEventListener('click', onDocumentClick);
    window.removeEventListener('popstate', onPopState);
});
</script>

<template>
    <div class="contents">
        <AdminShell
            :csrf="payload.csrf"
            :logout-url="payload.logoutUrl"
            :urls="payload.urls"
            :current="page"
        >
            <component :is="pages[page]" v-bind="pageProps" />
        </AdminShell>
        <AdminVedaChat v-if="enableChat" />
    </div>
</template>
