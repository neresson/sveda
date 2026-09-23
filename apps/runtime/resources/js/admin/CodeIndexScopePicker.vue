<script setup>
import { onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import CodeIndexScopeTreeNode from './CodeIndexScopeTreeNode.vue';

const props = defineProps({
    codeSourceId: { type: Number, required: true },
    csrf: { type: String, required: true },
    scopeUrl: { type: String, required: true },
    estimateUrl: { type: String, required: true },
    initialExcludePaths: { type: Array, default: () => [] },
    initialWorkspaceReady: { type: Boolean, default: false },
});

const excludePaths = defineModel('excludePaths', {
    type: Array,
    default: () => [],
});

const { t } = useI18n();

const ready = ref(props.initialWorkspaceReady);
const rootEntries = ref([]);
const loading = ref(false);
const loadError = ref('');
const footprintEstimate = ref(null);
const footprintLoading = ref(false);
const footprintError = ref('');
let pollTimer = null;

const isPathExcluded = (path) => {
    const normalized = String(path).replace(/\\/g, '/');
    return (excludePaths.value || []).some((excluded) => {
        const ex = String(excluded).replace(/\\/g, '/');
        return normalized === ex || normalized.startsWith(`${ex}/`);
    });
};

const setExcluded = (path, excluded) => {
    const normalized = String(path).replace(/\\/g, '/');
    let next = [...(excludePaths.value || [])];
    if (excluded) {
        next = next.filter((item) => {
            const ex = String(item).replace(/\\/g, '/');
            return ex !== normalized && !ex.startsWith(`${normalized}/`);
        });
        next.push(normalized);
    } else {
        next = next.filter((item) => {
            const ex = String(item).replace(/\\/g, '/');
            return ex !== normalized && !ex.startsWith(`${normalized}/`);
        });
    }
    excludePaths.value = next;
};

const fetchJson = async (url, options = {}) => {
    const response = await fetch(url, {
        credentials: 'same-origin',
        headers: {
            Accept: 'application/json',
            'X-CSRF-TOKEN': props.csrf,
            ...(options.headers ?? {}),
        },
        ...options,
    });
    const data = await response.json().catch(() => ({}));
    return { response, data };
};

const fetchStatus = async () => {
    loading.value = true;
    loadError.value = '';
    try {
        const { response, data } = await fetchJson(props.scopeUrl);
        if (!response.ok) {
            loadError.value = data.message || t('sources.scope_load_error');
            ready.value = false;
            return;
        }
        ready.value = !!data.ready;
        if (Array.isArray(data.exclude_paths) && data.exclude_paths.length) {
            excludePaths.value = data.exclude_paths;
        }
        if (ready.value && rootEntries.value.length === 0) {
            rootEntries.value = data.entries || [];
        }
    } catch {
        loadError.value = t('sources.scope_load_error');
        ready.value = false;
    } finally {
        loading.value = false;
    }
};

const estimateFootprint = async () => {
    footprintError.value = '';
    footprintEstimate.value = null;
    footprintLoading.value = true;
    try {
        const { response, data } = await fetchJson(props.estimateUrl, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ exclude_paths: excludePaths.value || [] }),
        });
        if (!response.ok) {
            footprintError.value = data.message || t('sources.scope_load_error');
            return;
        }
        footprintEstimate.value = data;
    } catch {
        footprintError.value = t('sources.scope_load_error');
    } finally {
        footprintLoading.value = false;
    }
};

watch(
    () => props.codeSourceId,
    () => {
        excludePaths.value = [...(props.initialExcludePaths || [])];
        rootEntries.value = [];
        void fetchStatus();
    },
);

watch(ready, (isReady) => {
    if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
    }
    if (!isReady) {
        pollTimer = setInterval(() => {
            void fetchStatus();
        }, 1500);
    }
}, { immediate: true });

onMounted(() => {
    void fetchStatus();
});

onUnmounted(() => {
    if (pollTimer) {
        clearInterval(pollTimer);
    }
});
</script>

<template>
    <div class="flex flex-col gap-3">
        <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.scope_title') }}</p>
        <p class="font-serif text-sm text-muted">{{ t('sources.scope_hint') }}</p>
        <p v-if="!ready" class="font-mono text-[11px] text-muted">{{ t('sources.scope_preparing') }}</p>
        <p v-if="loadError" class="font-mono text-sm">{{ loadError }}</p>
        <ul v-else-if="ready && rootEntries.length" class="max-h-80 overflow-auto border border-grid p-2">
            <CodeIndexScopeTreeNode
                v-for="entry in rootEntries"
                :key="`${entry.kind}-${entry.path}`"
                :node="entry"
                :csrf="csrf"
                :scope-url="scopeUrl"
                :is-path-excluded="isPathExcluded"
                :set-excluded="setExcluded"
            />
        </ul>
        <p v-else-if="ready && !loading" class="font-mono text-[11px] text-muted">{{ t('sources.scope_empty') }}</p>
        <p class="font-mono text-[11px] text-muted">
            {{ t('sources.scope_excluded_count', { count: (excludePaths || []).length }) }}
        </p>
        <button
            type="button"
            class="self-start border border-ink px-4 py-2 text-sm"
            :disabled="footprintLoading || !ready"
            @click="estimateFootprint"
        >
            {{ t('sources.estimate_footprint') }}
        </button>
        <p v-if="footprintError" class="font-mono text-sm">{{ footprintError }}</p>
        <div v-else-if="footprintEstimate" class="font-mono text-[11px] text-muted">
            <p>{{ t('sources.footprint_files_indexable', { count: footprintEstimate.files_indexable }) }}</p>
            <p>{{ t('sources.footprint_chunks', { count: footprintEstimate.chunks_total }) }}</p>
            <p>{{ t('sources.footprint_estimated_cost', { cost: footprintEstimate.estimated_cost_usd, rate: footprintEstimate.rate_usd_per_million_tokens }) }}</p>
        </div>
    </div>
</template>
