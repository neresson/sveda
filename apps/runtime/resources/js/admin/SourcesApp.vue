<script setup>
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import CodeIndexLocalFolderDialog from './CodeIndexLocalFolderDialog.vue';
import CodeIndexScopePicker from './CodeIndexScopePicker.vue';
import { useDocumentTitle } from './useDocumentTitle';

const props = defineProps({
    csrf: { type: String, required: true },
    saveUrl: { type: String, required: true },
    logoutUrl: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
    settings: { type: Object, default: () => ({}) },
    codeIndex: { type: Object, default: () => ({}) },
});

const { t, te } = useI18n();
useDocumentTitle('sources.document_title');

const sources = ref([]);
const localIndexingEnabled = ref(true);
const loading = ref(false);
const saving = ref(false);
const error = ref('');
const message = ref('');
const localPickerOpen = ref(false);
const localPathConfirmed = ref(false);
const editOpen = ref(false);
const deleteOpen = ref(false);
const pendingDelete = ref(null);
const editing = ref(null);
const scopeExcludePaths = ref([]);
const pollSnapshot = ref({});
let pollTimer = null;

const form = reactive({
    name: '',
    description: '',
    provider: 'local',
    local_absolute_path: '',
    git_remote_url: '',
    git_branch: 'main',
    git_clone_token: '',
});

const editForm = reactive({
    name: '',
    description: '',
});

const sourceUrl = (id, suffix = '') => {
    const base = String(props.codeIndex.sourceBase ?? '').replace(/\/$/, '');

    return `${base}/${id}${suffix}`;
};

const jsonHeaders = (extra = {}) => ({
    Accept: 'application/json',
    'X-CSRF-TOKEN': props.csrf,
    ...extra,
});

const parseJson = async (response) => response.json().catch(() => ({}));

const firstValidationMessage = (data) => {
    const errors = data?.errors;
    if (!errors || typeof errors !== 'object') {
        return data?.message || data?.error || '';
    }
    const first = Object.values(errors).flat()[0];

    return typeof first === 'string' ? first : '';
};

const translateError = (raw) => {
    if (!raw) {
        return '';
    }
    const key = `sources.index_errors.${raw}`;
    if (te(key)) {
        return t(key);
    }

    return raw;
};

const mergePoll = (source) => {
    const poll = pollSnapshot.value[source.id];
    if (!poll) {
        return source;
    }

    return {
        ...source,
        status: poll.status ?? source.status,
        indexing_progress: poll.indexing_progress ?? source.indexing_progress,
        indexing_phase: poll.indexing_phase ?? source.indexing_phase,
        metadata: poll.metadata ?? source.metadata,
        error_message: poll.error_message ?? source.error_message,
        last_indexed_at: poll.last_indexed_at ?? source.last_indexed_at,
        updated_at: poll.updated_at ?? source.updated_at,
    };
};

const displaySources = computed(() => sources.value.map(mergePoll));

const indexingActive = computed(() =>
    displaySources.value.some((source) => ['indexing', 'pending', 'configuring'].includes(source.status)),
);

const editingIsConfiguring = computed(() => editing.value?.status === 'configuring');

const localAddBlocked = computed(
    () => form.provider === 'local' && (!String(form.local_absolute_path).trim() || !localPathConfirmed.value),
);

const loadSources = async () => {
    if (!props.codeIndex.sources) {
        return;
    }
    loading.value = true;
    error.value = '';
    try {
        const response = await fetch(props.codeIndex.sources, {
            credentials: 'same-origin',
            headers: jsonHeaders(),
        });
        const data = await parseJson(response);
        if (!response.ok) {
            error.value = t('sources.load_failed');
            return;
        }
        sources.value = data.sources ?? [];
        localIndexingEnabled.value = data.localIndexingEnabled !== false;
        if (!localIndexingEnabled.value && form.provider === 'local') {
            form.provider = 'github';
        }
    } catch {
        error.value = t('sources.load_failed');
    } finally {
        loading.value = false;
    }
};

const fetchProgress = async () => {
    if (!props.codeIndex.progress) {
        return;
    }
    try {
        const response = await fetch(props.codeIndex.progress, {
            credentials: 'same-origin',
            headers: jsonHeaders(),
        });
        const data = await parseJson(response);
        if (!response.ok) {
            return;
        }
        const map = {};
        for (const source of data.sources ?? []) {
            map[source.id] = source;
        }
        pollSnapshot.value = map;
    } catch {
        // keep last snapshot
    }
};

const resetPollAndRefresh = async () => {
    pollSnapshot.value = {};
    await loadSources();
    await fetchProgress();
};

const resetForm = () => {
    form.name = '';
    form.description = '';
    form.provider = localIndexingEnabled.value ? 'local' : 'github';
    form.local_absolute_path = '';
    form.git_remote_url = '';
    form.git_branch = 'main';
    form.git_clone_token = '';
    localPathConfirmed.value = false;
};

const createSource = async () => {
    if (saving.value || localAddBlocked.value) {
        return;
    }
    saving.value = true;
    error.value = '';
    message.value = '';
    try {
        const payload = {
            name: form.name,
            description: form.description,
            provider: form.provider,
        };
        if (form.provider === 'local') {
            payload.local_absolute_path = form.local_absolute_path;
        } else {
            payload.git_remote_url = form.git_remote_url;
            payload.git_branch = form.git_branch;
            payload.git_clone_token = form.git_clone_token;
        }
        const response = await fetch(props.codeIndex.store, {
            method: 'POST',
            credentials: 'same-origin',
            headers: jsonHeaders({ 'Content-Type': 'application/json' }),
            body: JSON.stringify(payload),
        });
        const data = await parseJson(response);
        if (!response.ok) {
            error.value = translateError(firstValidationMessage(data)) || t('sources.create_failed');
            return;
        }
        sources.value = data.sources ?? sources.value;
        resetForm();
        if (data.source) {
            openEdit(data.source);
        }
        await fetchProgress();
    } catch {
        error.value = t('sources.create_failed');
    } finally {
        saving.value = false;
    }
};

const openEdit = (source) => {
    editing.value = source;
    editForm.name = source.name ?? '';
    editForm.description = source.description ?? '';
    scopeExcludePaths.value = Array.isArray(source.metadata?.exclude_paths) ? [...source.metadata.exclude_paths] : [];
    editOpen.value = true;
};

const saveEdit = async (startIndexing = false) => {
    if (!editing.value || saving.value) {
        return;
    }
    saving.value = true;
    error.value = '';
    try {
        const response = await fetch(sourceUrl(editing.value.id), {
            method: 'PATCH',
            credentials: 'same-origin',
            headers: jsonHeaders({ 'Content-Type': 'application/json' }),
            body: JSON.stringify({
                name: editForm.name,
                description: editForm.description,
                exclude_paths: scopeExcludePaths.value,
                start_indexing: startIndexing,
            }),
        });
        const data = await parseJson(response);
        if (!response.ok) {
            error.value = translateError(firstValidationMessage(data)) || t('sources.action_failed');
            return;
        }
        sources.value = data.sources ?? sources.value;
        editOpen.value = false;
        editing.value = null;
        await fetchProgress();
    } catch {
        error.value = t('sources.action_failed');
    } finally {
        saving.value = false;
    }
};

const cancelIndexing = async (id) => {
    error.value = '';
    try {
        const response = await fetch(sourceUrl(id, '/cancel-indexing'), {
            method: 'POST',
            credentials: 'same-origin',
            headers: jsonHeaders(),
        });
        const data = await parseJson(response);
        if (!response.ok) {
            error.value = translateError(data.error || data.message) || t('sources.action_failed');
        }
        await resetPollAndRefresh();
    } catch {
        error.value = t('sources.action_failed');
    }
};

const reindex = async (id) => {
    error.value = '';
    try {
        const response = await fetch(sourceUrl(id, '/reindex'), {
            method: 'POST',
            credentials: 'same-origin',
            headers: jsonHeaders(),
        });
        const data = await parseJson(response);
        if (!response.ok) {
            error.value = translateError(data.error || data.message) || t('sources.action_failed');
        }
        await resetPollAndRefresh();
    } catch {
        error.value = t('sources.action_failed');
    }
};

const confirmDelete = async () => {
    const source = pendingDelete.value;
    if (!source) {
        return;
    }
    saving.value = true;
    error.value = '';
    try {
        const response = await fetch(sourceUrl(source.id), {
            method: 'DELETE',
            credentials: 'same-origin',
            headers: jsonHeaders(),
        });
        const data = await parseJson(response);
        if (!response.ok) {
            error.value = t('sources.action_failed');
            return;
        }
        sources.value = data.sources ?? [];
        deleteOpen.value = false;
        pendingDelete.value = null;
    } catch {
        error.value = t('sources.action_failed');
    } finally {
        saving.value = false;
    }
};

const verifyLocalPath = async () => {
    const path = String(form.local_absolute_path || '').trim();
    if (!path || !props.codeIndex.localPreview) {
        return;
    }
    error.value = '';
    message.value = '';
    try {
        const response = await fetch(props.codeIndex.localPreview, {
            method: 'POST',
            credentials: 'same-origin',
            headers: jsonHeaders({ 'Content-Type': 'application/json' }),
            body: JSON.stringify({ path }),
        });
        const data = await parseJson(response);
        if (!response.ok) {
            localPathConfirmed.value = false;
            error.value = translateError(data.message) || t('sources.local_browse_error');
            return;
        }
        if (data.path) {
            form.local_absolute_path = data.path;
        }
        await nextTick();
        localPathConfirmed.value = true;
        message.value = t('sources.local_preview_files_count', { count: data.total_files ?? 0 });
    } catch {
        localPathConfirmed.value = false;
        error.value = t('sources.local_browse_error');
    }
};

const onLocalFolderConfirmed = async (path) => {
    form.local_absolute_path = path;
    error.value = '';
    await nextTick();
    localPathConfirmed.value = true;
};

const statusLabel = (source) => {
    const key = `sources.statuses.${source.status}`;

    return te(key) ? t(key) : source.status;
};

const phaseLabel = (phase) => {
    if (!phase) {
        return '';
    }
    const key = `sources.index_phase_${phase}`;

    return te(key) ? t(key) : phase;
};

const formatSourceError = (source) => {
    if (!source.error_message) {
        return '';
    }
    if (source.error_message === 'indexing_cancelled') {
        return t('sources.indexing_cancelled_message');
    }

    return translateError(source.error_message);
};

const providerDisplay = (source) =>
    source.provider === 'local' ? t('sources.provider_local') : t('sources.provider_github_https');

const chunksColumnDisplay = (source) => {
    const total = source.metadata?.chunks_total;
    if (total == null) {
        return '—';
    }
    if (['indexing', 'pending'].includes(source.status) && source.metadata?.chunks_embedded != null) {
        return `${source.metadata.chunks_embedded} / ${total}`;
    }

    return String(total);
};

const indexingProgressPercent = (source) => {
    if (source.status === 'pending') {
        return 4;
    }
    if (source.status !== 'indexing') {
        return source.indexing_progress ?? 0;
    }
    const total = source.metadata?.chunks_total ?? 0;
    if (source.indexing_phase === 'embedding' && total > 0) {
        const embedded = source.metadata?.chunks_embedded ?? 0;

        return Math.min(99, Math.max(74, 74 + Math.floor((25 * embedded) / total)));
    }

    return source.indexing_progress ?? 0;
};

const formatDate = (iso) => {
    if (!iso) {
        return '—';
    }
    try {
        return new Date(iso).toLocaleString();
    } catch {
        return iso;
    }
};

watch(
    () => form.local_absolute_path,
    () => {
        localPathConfirmed.value = false;
    },
);

watch(
    indexingActive,
    (active) => {
        if (pollTimer) {
            clearInterval(pollTimer);
            pollTimer = null;
        }
        if (active) {
            void fetchProgress();
            pollTimer = setInterval(() => {
                void fetchProgress();
            }, 1500);
        }
    },
    { immediate: true },
);

onMounted(() => {
    void loadSources();
});

onUnmounted(() => {
    if (pollTimer) {
        clearInterval(pollTimer);
    }
});
</script>

<template>
    <div class="contents">
        <div>
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('sources.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('sources.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">{{ t('sources.subtitle') }}</p>
        </div>

        <form class="flex flex-col gap-4 border border-ink p-4 lg:p-6" @submit.prevent="createSource">
            <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.add_source') }}</p>
            <div class="grid gap-4 md:grid-cols-2">
                <label class="flex flex-col gap-2 md:col-span-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.name') }}</span>
                    <input
                        v-model="form.name"
                        required
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                        :placeholder="t('sources.name_placeholder')"
                    >
                </label>
                <label class="flex flex-col gap-2 md:col-span-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.ai_context_label') }}</span>
                    <textarea
                        v-model="form.description"
                        rows="3"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                        :placeholder="t('sources.ai_context_hint')"
                    />
                </label>
                <label class="flex flex-col gap-2 md:col-span-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.provider') }}</span>
                    <select v-model="form.provider" class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none">
                        <option v-if="localIndexingEnabled" value="local">{{ t('sources.provider_local') }}</option>
                        <option value="github">{{ t('sources.provider_github_https') }}</option>
                    </select>
                    <span v-if="!localIndexingEnabled" class="font-mono text-[11px] text-muted">
                        {{ t('sources.local_indexing_disabled') }}
                    </span>
                </label>
                <template v-if="form.provider === 'local'">
                    <label class="flex flex-col gap-2 md:col-span-2">
                        <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.local_path') }}</span>
                        <div class="flex flex-wrap gap-2">
                            <input
                                v-model="form.local_absolute_path"
                                class="min-w-0 flex-1 border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                                :placeholder="t('sources.local_path_placeholder')"
                            >
                            <button
                                type="button"
                                class="border border-ink px-4 py-2 text-sm"
                                @click="localPickerOpen = true"
                            >
                                {{ t('sources.local_choose_folder') }}
                            </button>
                            <button
                                type="button"
                                class="border border-ink px-4 py-2 text-sm"
                                :disabled="!String(form.local_absolute_path).trim()"
                                @click="verifyLocalPath"
                            >
                                {{ t('sources.local_verify_path') }}
                            </button>
                        </div>
                        <span class="font-mono text-[11px] text-muted">{{ t('sources.local_path_hint') }}</span>
                        <span v-if="localAddBlocked" class="font-mono text-[11px] text-muted">
                            {{ t('sources.local_path_must_verify') }}
                        </span>
                    </label>
                </template>
                <template v-else>
                    <label class="flex flex-col gap-2 md:col-span-2">
                        <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.git_url') }}</span>
                        <input
                            v-model="form.git_remote_url"
                            class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                            :placeholder="t('sources.git_url_placeholder')"
                        >
                    </label>
                    <label class="flex flex-col gap-2">
                        <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.git_branch') }}</span>
                        <input
                            v-model="form.git_branch"
                            class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                        >
                    </label>
                    <label class="flex flex-col gap-2">
                        <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.git_token') }}</span>
                        <input
                            v-model="form.git_clone_token"
                            type="password"
                            autocomplete="off"
                            class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                        >
                        <span class="font-mono text-[11px] text-muted">{{ t('sources.git_token_hint') }}</span>
                    </label>
                </template>
            </div>
            <div class="flex justify-end">
                <button
                    type="submit"
                    class="veda-hover bg-ink px-7 py-3.5 text-sm font-semibold text-canvas hover:bg-ink/80 disabled:opacity-40 disabled:hover:bg-ink"
                    :disabled="saving || localAddBlocked"
                >
                    {{ saving ? t('sources.creating') : t('sources.create') }}
                </button>
            </div>
        </form>

        <p v-if="error" class="font-mono text-sm">{{ error }}</p>
        <p v-else-if="message" class="font-mono text-sm text-muted">{{ message }}</p>

        <div class="border border-ink">
            <div class="border-b border-ink px-4 py-3">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.sources_list_title') }}</p>
            </div>
            <p v-if="!displaySources.length" class="px-4 py-6 font-mono text-xs text-muted">
                {{ t('sources.sources_empty') }}
            </p>
            <article
                v-for="source in displaySources"
                :key="source.id"
                class="flex flex-col gap-3 border-b border-grid px-4 py-4 last:border-b-0"
            >
                <div class="flex flex-wrap items-start justify-between gap-3">
                    <div class="min-w-0">
                        <p class="truncate font-mono text-xs">{{ source.name }}</p>
                        <p class="truncate font-mono text-[11px] text-muted">{{ providerDisplay(source) }}</p>
                    </div>
                    <div class="flex flex-wrap items-center gap-4">
                        <button
                            type="button"
                            class="veda-hover font-mono text-[11px] tracking-[0.12em] hover:text-muted"
                            @click="openEdit(source)"
                        >
                            {{ source.status === 'configuring' ? t('sources.configure_indexing') : t('sources.edit_details') }}
                        </button>
                        <button
                            v-if="['indexing', 'pending', 'configuring'].includes(source.status)"
                            type="button"
                            class="veda-hover font-mono text-[11px] tracking-[0.12em] hover:text-muted"
                            @click="cancelIndexing(source.id)"
                        >
                            {{ t('sources.stop_indexing') }}
                        </button>
                        <button
                            v-else
                            type="button"
                            class="veda-hover font-mono text-[11px] tracking-[0.12em] hover:text-muted"
                            @click="reindex(source.id)"
                        >
                            {{ ['failed', 'cancelled'].includes(source.status) ? t('sources.retry_indexing') : t('sources.reindex') }}
                        </button>
                        <button
                            type="button"
                            class="veda-hover font-mono text-[11px] tracking-[0.12em] hover:text-muted"
                            @click="pendingDelete = source; deleteOpen = true"
                        >
                            {{ t('sources.delete') }}
                        </button>
                    </div>
                </div>
                <p class="font-mono text-[11px]">{{ statusLabel(source) }}</p>
                <div v-if="['indexing', 'pending'].includes(source.status)" class="flex flex-col gap-1">
                    <div class="h-1.5 w-full border border-ink">
                        <div class="h-full bg-ink" :style="{ width: `${indexingProgressPercent(source)}%` }" />
                    </div>
                    <p
                        v-if="source.indexing_phase === 'embedding' && (source.metadata?.chunks_total ?? 0) > 0"
                        class="font-mono text-[11px] text-muted"
                    >
                        {{ t('sources.embedding_chunks_progress', { embedded: source.metadata?.chunks_embedded ?? 0, total: source.metadata.chunks_total }) }}
                    </p>
                    <p v-else-if="source.indexing_phase" class="font-mono text-[11px] text-muted">
                        {{ phaseLabel(source.indexing_phase) }}
                    </p>
                </div>
                <p v-if="formatSourceError(source)" class="font-mono text-[11px]">{{ formatSourceError(source) }}</p>
                <p class="font-mono text-[11px] text-muted">
                    {{ t('sources.table_files') }}: {{ source.metadata?.files_indexed ?? '—' }}
                    · {{ t('sources.table_chunks') }}: {{ chunksColumnDisplay(source) }}
                    · {{ t('sources.last_indexed') }}: {{ formatDate(source.last_indexed_at) }}
                </p>
            </article>
        </div>

        <div
            v-if="editOpen && editing"
            class="fixed inset-0 z-50 flex items-center justify-center bg-ink/50 p-4"
        >
            <div class="flex max-h-[90vh] w-full max-w-2xl flex-col overflow-auto border border-ink bg-canvas">
                <div class="border-b border-grid p-4">
                    <p class="font-mono text-[11px] tracking-[0.18em] text-muted">
                        {{ editingIsConfiguring ? t('sources.configure_indexing') : t('sources.edit_source') }}
                    </p>
                    <p class="mt-2 font-serif text-sm text-muted">{{ t('sources.edit_source_hint') }}</p>
                </div>
                <div class="flex flex-col gap-4 p-4">
                    <label class="flex flex-col gap-2">
                        <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.name') }}</span>
                        <input v-model="editForm.name" class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none">
                    </label>
                    <label class="flex flex-col gap-2">
                        <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('sources.ai_context_label') }}</span>
                        <textarea
                            v-model="editForm.description"
                            rows="4"
                            class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                        />
                    </label>
                    <CodeIndexScopePicker
                        v-model:exclude-paths="scopeExcludePaths"
                        :code-source-id="editing.id"
                        :csrf="csrf"
                        :scope-url="sourceUrl(editing.id, '/scope')"
                        :estimate-url="sourceUrl(editing.id, '/estimate-footprint')"
                        :initial-exclude-paths="scopeExcludePaths"
                        :initial-workspace-ready="editing.provider === 'local' || !!editing.metadata?.workspace_ready"
                    />
                </div>
                <div class="flex flex-wrap justify-end gap-2 border-t border-grid p-4">
                    <button type="button" class="border border-ink px-4 py-2 text-sm" @click="editOpen = false">
                        {{ t('common.cancel') }}
                    </button>
                    <button
                        type="button"
                        class="border border-ink px-4 py-2 text-sm"
                        :disabled="saving"
                        @click="saveEdit(false)"
                    >
                        {{ t('sources.save') }}
                    </button>
                    <button
                        v-if="editingIsConfiguring"
                        type="button"
                        class="veda-hover bg-ink px-4 py-2 text-sm font-semibold text-canvas hover:bg-ink/80 disabled:opacity-40"
                        :disabled="saving"
                        @click="saveEdit(true)"
                    >
                        {{ t('sources.start_indexing') }}
                    </button>
                </div>
            </div>
        </div>

        <div
            v-if="deleteOpen && pendingDelete"
            class="fixed inset-0 z-50 flex items-center justify-center bg-ink/50 p-4"
        >
            <div class="w-full max-w-md border border-ink bg-canvas p-4">
                <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('sources.delete_confirm_title') }}</p>
                <p class="mt-3 font-serif text-sm">
                    {{ t('sources.delete_confirm_description', { name: pendingDelete.name }) }}
                </p>
                <div class="mt-6 flex justify-end gap-2">
                    <button type="button" class="border border-ink px-4 py-2 text-sm" @click="deleteOpen = false">
                        {{ t('common.cancel') }}
                    </button>
                    <button
                        type="button"
                        class="veda-hover bg-ink px-4 py-2 text-sm font-semibold text-canvas hover:bg-ink/80"
                        :disabled="saving"
                        @click="confirmDelete"
                    >
                        {{ t('sources.delete') }}
                    </button>
                </div>
            </div>
        </div>

        <CodeIndexLocalFolderDialog
            v-model:open="localPickerOpen"
            :csrf="csrf"
            :browse-url="codeIndex.localBrowse"
            :preview-url="codeIndex.localPreview"
        />
    </div>
</template>
