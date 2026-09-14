<script setup>
import { computed, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import AdminShell from './AdminShell.vue';
import { useDocumentTitle } from './useDocumentTitle';

const props = defineProps({
    csrf: { type: String, required: true },
    saveUrl: { type: String, required: true },
    logoutUrl: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
    settings: { type: Object, default: () => ({}) },
});

const emptyDraft = () => ({
    id: '',
    label: '',
    protocol: 'responses',
    api_model: '',
    url: '',
    key: '',
    thinking: false,
    vision: false,
    aliases: '',
    preset: '',
});

const toDraft = (model) => {
    const aliases = Array.isArray(model.aliases)
        ? model.aliases.join(', ')
        : String(model.aliases ?? '');

    return {
        id: String(model.id ?? ''),
        label: String(model.label ?? ''),
        protocol: model.protocol === 'anthropic' ? 'anthropic' : 'responses',
        api_model: String(model.api_model ?? model.apiModel ?? ''),
        url: String(model.url ?? ''),
        key: '',
        thinking: Boolean(model.thinking),
        vision: Boolean(model.vision),
        aliases,
        preset: model.preset ?? '',
    };
};

const models = ref(Array.isArray(props.settings.models) ? [...props.settings.models] : []);
const draft = reactive(emptyDraft());
const editingId = ref('');
const saving = ref(false);
const message = ref('');
const error = ref('');

const isEditing = computed(() => editingId.value !== '');
const canSubmit = computed(() => draft.id.trim() !== '' && draft.label.trim() !== '');
const editingHasKey = computed(() => {
    if (!isEditing.value) {
        return false;
    }

    const model = models.value.find((item) => item.id === editingId.value);

    return Boolean(model?.key);
});

const { t } = useI18n();
useDocumentTitle('models.document_title');

const modelFlags = (model) =>
    [model.thinking ? t('models.thinking') : null, model.vision ? t('models.vision') : null].filter(Boolean).join(' · ');

const resetDraft = () => {
    editingId.value = '';
    Object.assign(draft, emptyDraft());
};

const serialize = (model) => {
    const aliases = Array.isArray(model.aliases)
        ? model.aliases
        : String(model.aliases ?? '')
            .split(',')
            .map((alias) => alias.trim())
            .filter(Boolean);

    return {
        id: String(model.id ?? '').trim(),
        label: String(model.label ?? '').trim(),
        protocol: model.protocol === 'anthropic' ? 'anthropic' : 'responses',
        api_model: String(model.api_model ?? model.apiModel ?? '').trim(),
        url: String(model.url ?? '').trim(),
        key: String(model.key ?? ''),
        thinking: Boolean(model.thinking),
        vision: Boolean(model.vision),
        aliases,
        preset: model.preset ?? '',
    };
};

const saveModels = async (next) => {
    saving.value = true;
    error.value = '';
    message.value = '';

    try {
        const response = await fetch(props.saveUrl, {
            method: 'POST',
            credentials: 'same-origin',
            headers: {
                Accept: 'application/json',
                'Content-Type': 'application/json',
                'X-CSRF-TOKEN': props.csrf,
            },
            body: JSON.stringify({
                models: next.map(serialize).filter((model) => model.id !== ''),
            }),
        });

        const body = await response.json().catch(() => ({}));

        if (!response.ok) {
            error.value = body.message ?? t('common.save_failed');
            return false;
        }

        models.value = Array.isArray(body.models) ? body.models : next;
        message.value = t('common.saved');
        return true;
    } catch {
        error.value = t('common.save_failed');
        return false;
    } finally {
        saving.value = false;
    }
};

const submit = async () => {
    if (!canSubmit.value || saving.value) {
        return;
    }

    const id = draft.id.trim();
    const taken = models.value.some((model) => model.id === id && model.id !== editingId.value);
    if (taken) {
        error.value = t('models.id_taken');
        return;
    }

    const payload = { ...draft };
    const next = isEditing.value
        ? models.value.map((model) => (
            model.id === editingId.value
                ? { ...payload, preset: payload.preset || model.preset }
                : model
        ))
        : [...models.value, payload];

    const saved = await saveModels(next);
    if (saved) {
        resetDraft();
        message.value = t('common.saved');
    }
};

const startEdit = (model) => {
    if (saving.value) {
        return;
    }

    editingId.value = String(model.id);
    Object.assign(draft, toDraft(model));
    error.value = '';
    message.value = '';
};

const cancelEdit = () => {
    if (saving.value) {
        return;
    }

    resetDraft();
    error.value = '';
    message.value = '';
};

const removeModel = async (id) => {
    if (saving.value) {
        return;
    }

    const saved = await saveModels(models.value.filter((model) => model.id !== id));
    if (saved && editingId.value === id) {
        resetDraft();
        message.value = t('common.saved');
    }
};
</script>

<template>
    <AdminShell :csrf="csrf" :logout-url="logoutUrl" :urls="urls" current="models">
        <div>
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('models.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('models.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">{{ t('models.subtitle') }}</p>
        </div>

        <form class="flex flex-col gap-4 border border-ink p-4 lg:p-6" @submit.prevent="submit">
            <p v-if="isEditing" class="font-mono text-[11px] tracking-[0.14em] text-muted">
                {{ t('models.editing', { id: editingId }) }}
            </p>
            <div class="grid gap-4 md:grid-cols-2">
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('models.id') }}</span>
                    <input v-model="draft.id" name="model_id" class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none" required>
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('models.label') }}</span>
                    <input v-model="draft.label" name="model_label" class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none" required>
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('models.protocol') }}</span>
                    <select v-model="draft.protocol" name="model_protocol" class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none">
                        <option value="responses">responses</option>
                        <option value="anthropic">anthropic</option>
                    </select>
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('models.api_model') }}</span>
                    <input v-model="draft.api_model" name="model_api_model" class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none">
                </label>
                <label class="flex flex-col gap-2 md:col-span-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('models.url') }}</span>
                    <input v-model="draft.url" name="model_url" class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none">
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('models.api_key') }}</span>
                    <input v-model="draft.key" name="model_key" type="password" autocomplete="off" class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none">
                    <span v-if="editingHasKey" class="font-mono text-[11px] text-muted">{{ t('models.keep_key') }}</span>
                </label>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('models.aliases') }}</span>
                    <input v-model="draft.aliases" name="model_aliases" class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none">
                </label>
            </div>
            <div class="flex flex-col gap-4 sm:flex-row sm:items-center">
                <div class="flex items-center gap-6">
                    <label class="flex items-center gap-2 font-mono text-xs">
                        <input v-model="draft.thinking" type="checkbox" class="size-3.5 border border-ink accent-ink">
                        {{ t('models.thinking') }}
                    </label>
                    <label class="flex items-center gap-2 font-mono text-xs">
                        <input v-model="draft.vision" type="checkbox" class="size-3.5 border border-ink accent-ink">
                        {{ t('models.vision') }}
                    </label>
                </div>
                <div class="flex items-center gap-4 sm:ml-auto">
                    <button
                        v-if="isEditing"
                        type="button"
                        class="font-mono text-[11px] tracking-[0.16em]"
                        :disabled="saving"
                        @click="cancelEdit"
                    >
                        {{ t('common.cancel') }}
                    </button>
                    <button
                        type="submit"
                        class="bg-ink px-7 py-3.5 text-sm font-semibold text-canvas disabled:opacity-40"
                        :disabled="!canSubmit || saving"
                    >
                        {{ isEditing ? t('common.save') : t('common.add') }}
                    </button>
                </div>
            </div>
        </form>

        <p v-if="error" class="font-mono text-sm">{{ error }}</p>
        <p v-else-if="message" class="font-mono text-sm text-muted">{{ message }}</p>

        <div class="border border-ink lg:hidden">
            <article
                v-for="model in models"
                :key="`mobile-${model.id}`"
                class="flex flex-col gap-3 border-b border-grid px-4 py-4 last:border-b-0"
                :class="model.id === editingId ? 'bg-grid/50' : ''"
            >
                <div class="min-w-0">
                    <p class="truncate font-mono text-xs">{{ model.id }}</p>
                    <p class="truncate font-mono text-[11px] text-muted">{{ model.label }}</p>
                </div>
                <div class="flex items-center justify-between gap-3">
                    <p class="font-mono text-[11px] text-muted">
                        {{ model.protocol }}<template v-if="modelFlags(model)"> · {{ modelFlags(model) }}</template>
                    </p>
                    <div class="flex items-center gap-4">
                        <button
                            type="button"
                            class="font-mono text-[11px] tracking-[0.12em]"
                            :disabled="saving"
                            @click="startEdit(model)"
                        >
                            {{ t('common.edit') }}
                        </button>
                        <button
                            type="button"
                            class="font-mono text-[11px] tracking-[0.12em]"
                            :disabled="saving"
                            @click="removeModel(model.id)"
                        >
                            {{ t('common.remove') }}
                        </button>
                    </div>
                </div>
            </article>
            <p v-if="models.length === 0" class="px-4 py-6 font-mono text-xs text-muted">{{ t('models.empty') }}</p>
        </div>

        <div class="hidden border border-ink lg:block">
            <div class="flex gap-4 border-b border-ink px-4 py-3 font-mono text-[11px] tracking-[0.14em] text-muted">
                <span class="flex-1">{{ t('models.model') }}</span>
                <span class="w-24">{{ t('models.protocol') }}</span>
                <span class="w-28">{{ t('models.flags') }}</span>
                <span class="w-32"></span>
            </div>
            <div
                v-for="model in models"
                :key="model.id"
                class="flex items-center gap-4 border-b border-grid px-4 py-3.5 last:border-b-0"
                :class="model.id === editingId ? 'bg-grid/50' : ''"
            >
                <div class="min-w-0 flex-1">
                    <p class="truncate font-mono text-xs">{{ model.id }}</p>
                    <p class="truncate font-mono text-[11px] text-muted">{{ model.label }}</p>
                </div>
                <span class="w-24 font-mono text-[11px] text-muted">{{ model.protocol }}</span>
                <span class="w-28 font-mono text-[11px]">
                    {{ modelFlags(model) || '—' }}
                </span>
                <div class="flex w-32 shrink-0 justify-end gap-4">
                    <button
                        type="button"
                        class="font-mono text-[11px] tracking-[0.12em]"
                        :disabled="saving"
                        @click="startEdit(model)"
                    >
                        {{ t('common.edit') }}
                    </button>
                    <button
                        type="button"
                        class="font-mono text-[11px] tracking-[0.12em]"
                        :disabled="saving"
                        @click="removeModel(model.id)"
                    >
                        {{ t('common.remove') }}
                    </button>
                </div>
            </div>
            <p v-if="models.length === 0" class="px-4 py-6 font-mono text-xs text-muted">{{ t('models.empty') }}</p>
        </div>
    </AdminShell>
</template>
