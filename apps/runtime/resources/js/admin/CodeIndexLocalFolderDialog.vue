<script setup>
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

const props = defineProps({
    open: { type: Boolean, default: false },
    csrf: { type: String, required: true },
    browseUrl: { type: String, default: '' },
    previewUrl: { type: String, default: '' },
});

const emit = defineEmits(['update:open', 'confirmed']);

const { t } = useI18n();

const step = ref('browse');
const loading = ref(false);
const browseError = ref('');
const needsAnchor = ref(false);
const currentPath = ref(null);
const parentPath = ref(null);
const entries = ref([]);
const anchorInput = ref('');
const confirmPath = ref(null);
const previewCount = ref(null);

const resetState = () => {
    step.value = 'browse';
    loading.value = false;
    browseError.value = '';
    needsAnchor.value = false;
    currentPath.value = null;
    parentPath.value = null;
    entries.value = [];
    anchorInput.value = '';
    confirmPath.value = null;
    previewCount.value = null;
};

const loadBrowse = async (path) => {
    loading.value = true;
    browseError.value = '';
    try {
        const url = new URL(props.browseUrl, window.location.origin);
        if (path) {
            url.searchParams.set('path', path);
        }
        const response = await fetch(url.toString(), {
            credentials: 'same-origin',
            headers: {
                Accept: 'application/json',
                'X-CSRF-TOKEN': props.csrf,
            },
        });
        const data = await response.json().catch(() => ({}));
        if (!response.ok) {
            browseError.value = data.message || t('sources.local_browse_error');
            entries.value = [];
            return;
        }
        needsAnchor.value = data.needs_anchor === true;
        currentPath.value = data.current_path;
        parentPath.value = data.parent_path;
        entries.value = data.entries || [];
        if (needsAnchor.value) {
            anchorInput.value = '';
        }
    } catch {
        browseError.value = t('sources.local_browse_error');
        entries.value = [];
    } finally {
        loading.value = false;
    }
};

const openAnchorPath = async () => {
    const path = anchorInput.value.trim();
    if (!path) {
        browseError.value = t('sources.local_browse_anchor_required');
        return;
    }
    await loadBrowse(path);
};

const goParent = () => {
    if (parentPath.value) {
        void loadBrowse(parentPath.value);
    }
};

const canPreviewHere = () => typeof currentPath.value === 'string' && currentPath.value !== '';

const goToConfirm = async () => {
    if (!canPreviewHere()) {
        return;
    }
    confirmPath.value = currentPath.value;
    previewCount.value = null;
    step.value = 'confirm';
    loading.value = true;
    try {
        const response = await fetch(props.previewUrl, {
            method: 'POST',
            credentials: 'same-origin',
            headers: {
                Accept: 'application/json',
                'Content-Type': 'application/json',
                'X-CSRF-TOKEN': props.csrf,
            },
            body: JSON.stringify({ path: confirmPath.value }),
        });
        const data = await response.json().catch(() => ({}));
        if (response.ok) {
            previewCount.value = data.total_files ?? 0;
        }
    } finally {
        loading.value = false;
    }
};

const confirm = () => {
    if (!confirmPath.value) {
        return;
    }
    emit('confirmed', confirmPath.value);
    emit('update:open', false);
};

const close = () => {
    emit('update:open', false);
};

watch(
    () => props.open,
    (open) => {
        if (open) {
            resetState();
            void loadBrowse('');
        }
    },
);
</script>

<template>
    <div
        v-if="open"
        class="fixed inset-0 z-50 flex items-center justify-center bg-ink/50 p-4"
    >
        <div class="flex max-h-[90vh] w-full max-w-xl flex-col border border-ink bg-canvas">
            <div class="border-b border-grid p-4">
                <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('sources.local_browse_title') }}</p>
                <p class="mt-2 font-serif text-sm text-muted">{{ t('sources.local_browse_description') }}</p>
            </div>

            <div v-if="step === 'browse'" class="flex min-h-0 flex-1 flex-col gap-3 overflow-auto p-4">
                <label v-if="needsAnchor || !currentPath" class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.local_browse_anchor_label') }}</span>
                    <div class="flex gap-2">
                        <input
                            v-model="anchorInput"
                            type="text"
                            class="min-w-0 flex-1 border border-ink bg-canvas px-3 py-2 font-mono text-sm outline-none"
                            :placeholder="t('sources.local_browse_anchor_placeholder')"
                            @keydown.enter.prevent="openAnchorPath"
                        >
                        <button
                            type="button"
                            class="sveda-hover bg-ink px-4 py-2 text-sm font-semibold text-canvas hover:bg-ink/80"
                            :disabled="loading"
                            @click="openAnchorPath"
                        >
                            {{ t('sources.local_browse_go') }}
                        </button>
                    </div>
                </label>

                <div v-if="currentPath" class="flex items-center justify-between gap-2">
                    <p class="truncate font-mono text-[11px] text-muted">{{ currentPath }}</p>
                    <button
                        v-if="parentPath"
                        type="button"
                        class="font-mono text-[11px] tracking-[0.12em] underline"
                        @click="goParent"
                    >
                        {{ t('sources.local_browse_up') }}
                    </button>
                </div>

                <ul v-if="entries.length" class="flex flex-col border border-grid">
                    <li v-for="entry in entries" :key="entry.path">
                        <button
                            type="button"
                            class="sveda-hover w-full border-b border-grid px-3 py-2 text-left font-mono text-sm last:border-b-0 hover:bg-grid"
                            @click="loadBrowse(entry.path)"
                        >
                            {{ entry.name }}
                        </button>
                    </li>
                </ul>
                <p v-else-if="currentPath && !loading" class="font-mono text-[11px] text-muted">
                    {{ t('sources.local_browse_empty') }}
                </p>
                <p v-if="browseError" class="font-mono text-sm">{{ browseError }}</p>
            </div>

            <div v-else class="flex flex-col gap-3 p-4">
                <p class="font-mono text-sm">{{ confirmPath }}</p>
                <p class="font-serif text-sm text-muted">{{ t('sources.local_preview_confirm_question') }}</p>
                <p v-if="previewCount !== null" class="font-mono text-[11px] text-muted">
                    {{ t('sources.local_preview_files_count', { count: previewCount }) }}
                </p>
            </div>

            <div class="flex justify-end gap-2 border-t border-grid p-4">
                <button
                    type="button"
                    class="border border-ink px-4 py-2 text-sm"
                    @click="step === 'confirm' ? (step = 'browse') : close()"
                >
                    {{ step === 'confirm' ? t('sources.local_browse_back') : t('common.close') }}
                </button>
                <button
                    v-if="step === 'browse'"
                    type="button"
                    class="sveda-hover bg-ink px-4 py-2 text-sm font-semibold text-canvas hover:bg-ink/80 disabled:opacity-40"
                    :disabled="!canPreviewHere() || loading"
                    @click="goToConfirm"
                >
                    {{ t('sources.local_browse_continue') }}
                </button>
                <button
                    v-else
                    type="button"
                    class="sveda-hover bg-ink px-4 py-2 text-sm font-semibold text-canvas hover:bg-ink/80"
                    @click="confirm"
                >
                    {{ t('sources.local_browse_confirm') }}
                </button>
            </div>
        </div>
    </div>
</template>
