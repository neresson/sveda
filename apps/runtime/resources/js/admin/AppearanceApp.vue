<script setup>
import { computed, onUnmounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
    applyVedaAppearance,
    formatRadiusPx,
    parseRadiusPx,
    sanitizeVedaLauncherImage,
    VEDA_DEFAULT_LAUNCHER_ICON,
    VEDA_LAUNCHER_ICON_IDS,
    VEDA_LAUNCHER_IMAGE_MAX_BYTES,
    vedaLauncherIconComponent,
} from '@veda-ai/vue';
import { useDocumentTitle } from './useDocumentTitle';

const props = defineProps({
    csrf: { type: String, required: true },
    saveUrl: { type: String, required: true },
    logoutUrl: { type: String, required: true },
    urls: { type: Object, default: () => ({}) },
    settings: { type: Object, default: () => ({}) },
    appearancePresets: { type: Object, default: () => ({}) },
});

const { t } = useI18n();
useDocumentTitle('appearance.document_title');

const saving = ref(false);
const message = ref('');
const error = ref('');
const lookPresets = [
    { id: 'default', color: 'default', radius: '0px' },
    { id: 'lms', color: 'lms', radius: '8px' },
    { id: 'rounded', color: 'forest', radius: '20px' },
];
const colorIds = ['default', 'lms', 'ocean', 'forest', 'sunset', 'sand'];
const launcherIcons = VEDA_LAUNCHER_ICON_IDS;
const launcherImageTypes = ['image/png', 'image/jpeg', 'image/webp', 'image/gif'];
const previewModes = ['light', 'dark'];

const tokenHsl = (presetId, mode, key, fallback) => {
    const preset = props.appearancePresets?.[presetId] ?? {};
    const tokens = (mode === 'dark' ? preset.dark_tokens : preset.tokens) ?? {};

    return `hsl(${tokens[key] ?? fallback})`;
};

const surfaceStyle = (presetId, mode) => ({
    backgroundColor: tokenHsl(presetId, mode, 'background', mode === 'dark' ? '222.2 47.4% 11.2%' : '210 20% 98%'),
    color: tokenHsl(presetId, mode, 'foreground', mode === 'dark' ? '210 40% 98%' : '0 0% 9%'),
});

const brandStyle = (presetId, mode, radius) => ({
    backgroundColor: tokenHsl(presetId, mode, 'brand', mode === 'dark' ? '210 40% 98%' : '0 0% 9%'),
    borderRadius: radius,
});

const cardStyle = (presetId, mode, radius) => ({
    backgroundColor: tokenHsl(presetId, mode, 'card', mode === 'dark' ? '222.2 47.4% 14%' : '0 0% 100%'),
    borderColor: tokenHsl(presetId, mode, 'border', mode === 'dark' ? '217.2 32.6% 17.5%' : '220 13% 91%'),
    borderRadius: radius,
});

const fromDocument = (document) => {
    const appearance = document?.appearance ?? {};
    const preset = colorIds.includes(appearance.preset)
        ? appearance.preset
        : appearance.preset === 'custom'
            ? 'custom'
            : 'default';

    return {
        preset,
        radius: String(appearance.radius ?? '0px'),
        label: String(appearance.launcher?.label ?? ''),
        icon: launcherIcons.includes(appearance.launcher?.icon)
            ? appearance.launcher.icon
            : VEDA_DEFAULT_LAUNCHER_ICON,
        image: sanitizeVedaLauncherImage(appearance.launcher?.image),
    };
};

const form = reactive(fromDocument(props.settings));
const savedAppearance = ref(props.settings?.appearance ?? null);

const colorPreset = computed(() => {
    if (form.preset === 'custom') {
        return savedAppearance.value;
    }

    return props.appearancePresets?.[form.preset] ?? props.appearancePresets?.default ?? null;
});

const liveAppearance = computed(() => {
    const launcher = {
        label: form.label,
        icon: form.icon,
        image: form.image,
    };
    const preset = colorPreset.value;
    if (!preset) {
        return { radius: form.radius, launcher };
    }

    return {
        ...preset,
        radius: form.radius,
        launcher,
    };
});

const activeLook = computed(
    () => lookPresets.find((look) => look.color === form.preset && look.radius === form.radius) ?? null,
);

const canSave = computed(() => colorIds.includes(form.preset) || form.preset === 'custom');

watch(
    () => props.settings?.appearance,
    (value) => {
        savedAppearance.value = value ?? null;
    },
);

watch(
    liveAppearance,
    (value) => {
        applyVedaAppearance(value);
    },
    { immediate: true, deep: true },
);

onUnmounted(() => {
    applyVedaAppearance(savedAppearance.value);
});

const applyLook = (look) => {
    form.preset = look.color;
    form.radius = look.radius;
};

const selectColor = (id) => {
    form.preset = id;
};

const setRadius = (value) => {
    form.radius = formatRadiusPx(Number(value));
};

const selectLauncherIcon = (id) => {
    form.icon = id;
    form.image = '';
};

const clearLauncherImage = () => {
    form.image = '';
};

const onLauncherImage = (event) => {
    const input = event.target;
    const file = input.files?.[0] ?? null;
    input.value = '';
    if (!file) {
        return;
    }

    if (!launcherImageTypes.includes(file.type)) {
        error.value = t('appearance.launcher_image_type');
        return;
    }

    if (file.size > VEDA_LAUNCHER_IMAGE_MAX_BYTES) {
        error.value = t('appearance.launcher_image_size');
        return;
    }

    const reader = new FileReader();
    reader.onload = () => {
        const result = typeof reader.result === 'string' ? sanitizeVedaLauncherImage(reader.result) : '';
        if (result === '') {
            error.value = t('appearance.launcher_image_type');
            return;
        }

        form.image = result;
        error.value = '';
        message.value = '';
    };
    reader.readAsDataURL(file);
};

const savePayload = () => {
    const launcher = {
        label: form.label.trim(),
        icon: form.icon,
        image: form.image,
    };

    if (form.preset === 'custom') {
        return {
            appearance: {
                preset: 'custom',
                radius: form.radius,
                tokens: savedAppearance.value?.tokens ?? {},
                dark_tokens: savedAppearance.value?.dark_tokens ?? {},
                launcher,
            },
        };
    }

    return {
        appearance: {
            preset: form.preset,
            radius: form.radius,
            launcher,
        },
    };
};

const save = async () => {
    if (saving.value || !canSave.value) {
        return;
    }

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
            body: JSON.stringify(savePayload()),
        });

        const body = await response.json().catch(() => ({}));

        if (!response.ok) {
            error.value = body.message ?? t('common.save_failed');
            return;
        }

        Object.assign(form, fromDocument(body));
        savedAppearance.value = body.appearance ?? savedAppearance.value;
        message.value = t('common.saved');
    } catch {
        error.value = t('common.save_failed');
    } finally {
        saving.value = false;
    }
};
</script>

<template>
    <div class="contents">
        <div>
            <p class="font-mono text-[11px] tracking-[0.18em] text-muted">{{ t('appearance.eyebrow') }}</p>
            <h1 class="mt-2 text-4xl font-bold tracking-tighter sm:text-5xl lg:text-6xl">{{ t('appearance.title') }}</h1>
            <p class="mt-2 font-serif text-base text-muted lg:text-lg">{{ t('appearance.subtitle') }}</p>
        </div>

        <form class="flex flex-col gap-6 border border-ink p-4 lg:p-6" @submit.prevent="save">
            <div class="flex flex-col gap-3">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('appearance.looks') }}</p>
                <div class="grid gap-3 md:grid-cols-3">
                    <button
                        v-for="look in lookPresets"
                        :key="look.id"
                        type="button"
                        class="flex flex-col overflow-hidden border text-left"
                        :class="activeLook?.id === look.id ? 'border-ink' : 'border-grid hover:border-ink'"
                        @click="applyLook(look)"
                    >
                        <span class="grid h-20 grid-cols-2">
                            <span
                                v-for="mode in previewModes"
                                :key="mode"
                                class="flex items-end justify-between gap-2 px-3 py-3"
                                :style="surfaceStyle(look.color, mode)"
                            >
                                <span class="h-8 w-10" :style="brandStyle(look.color, mode, look.radius)" />
                                <span class="h-8 min-w-0 flex-1 border" :style="cardStyle(look.color, mode, look.radius)" />
                            </span>
                        </span>
                        <span
                            class="flex flex-col gap-1 px-4 py-3"
                            :class="activeLook?.id === look.id ? 'bg-ink text-canvas' : 'bg-canvas text-ink'"
                        >
                            <span class="font-mono text-xs tracking-[0.12em]">{{ t(`appearance.look_${look.id}`) }}</span>
                            <span class="font-serif text-sm" :class="activeLook?.id === look.id ? 'text-canvas/80' : 'text-muted'">
                                {{ t(`appearance.look_${look.id}_hint`) }}
                            </span>
                        </span>
                    </button>
                </div>
            </div>

            <div class="flex flex-col gap-3 border-t border-grid pt-4">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('appearance.colors') }}</p>
                <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                    <button
                        v-for="id in colorIds"
                        :key="id"
                        type="button"
                        class="flex flex-col overflow-hidden border text-left"
                        :class="form.preset === id ? 'border-ink' : 'border-grid hover:border-ink'"
                        @click="selectColor(id)"
                    >
                        <span class="grid h-16 grid-cols-2">
                            <span
                                v-for="mode in previewModes"
                                :key="mode"
                                class="flex items-end justify-between gap-2 px-3 py-3"
                                :style="surfaceStyle(id, mode)"
                            >
                                <span class="h-7 w-9" :style="brandStyle(id, mode, form.radius)" />
                                <span class="h-7 min-w-0 flex-1 border" :style="cardStyle(id, mode, form.radius)" />
                            </span>
                        </span>
                        <span
                            class="flex flex-col gap-1 px-4 py-3"
                            :class="form.preset === id ? 'bg-ink text-canvas' : 'bg-canvas text-ink'"
                        >
                            <span class="font-mono text-xs tracking-[0.12em]">{{ t(`appearance.color_${id}`) }}</span>
                            <span class="font-serif text-sm" :class="form.preset === id ? 'text-canvas/80' : 'text-muted'">
                                {{ t(`appearance.color_${id}_hint`) }}
                            </span>
                        </span>
                    </button>
                </div>
                <p v-if="form.preset === 'custom'" class="font-mono text-[11px] text-muted">{{ t('appearance.preset_custom') }}</p>
            </div>

            <label class="flex flex-col gap-2 border-t border-grid pt-4">
                <span class="font-mono text-[11px] tracking-[0.14em] text-muted">
                    {{ t('appearance.radius') }} · {{ form.radius }}
                </span>
                <input
                    type="range"
                    min="0"
                    max="32"
                    step="1"
                    :value="parseRadiusPx(form.radius)"
                    class="w-full accent-ink"
                    @input="setRadius($event.target.value)"
                >
                <span class="font-mono text-[11px] text-muted">{{ t('appearance.radius_hint') }}</span>
            </label>

            <div class="flex flex-col gap-4 border-t border-grid pt-4">
                <p class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('appearance.launcher') }}</p>
                <label class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('appearance.launcher_label') }}</span>
                    <input
                        v-model="form.label"
                        type="text"
                        maxlength="64"
                        :placeholder="t('appearance.launcher_label_placeholder')"
                        class="border border-ink bg-canvas px-3.5 py-3 font-mono text-sm outline-none"
                    >
                    <span class="font-mono text-[11px] text-muted">{{ t('appearance.launcher_label_hint') }}</span>
                </label>
                <div class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('appearance.launcher_icon') }}</span>
                    <div class="grid grid-cols-6 gap-2 sm:grid-cols-12">
                        <button
                            v-for="id in launcherIcons"
                            :key="id"
                            type="button"
                            class="flex h-11 items-center justify-center border"
                            :class="form.image === '' && form.icon === id ? 'border-ink bg-ink text-canvas' : 'border-grid hover:border-ink'"
                            :title="id"
                            @click="selectLauncherIcon(id)"
                        >
                            <component :is="vedaLauncherIconComponent(id)" class="h-5 w-5" />
                        </button>
                    </div>
                    <span class="font-mono text-[11px] text-muted">{{ t('appearance.launcher_icon_hint') }}</span>
                </div>
                <div class="flex flex-col gap-2">
                    <span class="font-mono text-[11px] tracking-[0.14em] text-muted">{{ t('appearance.launcher_image') }}</span>
                    <div class="flex flex-wrap items-center gap-3">
                        <img
                            v-if="form.image"
                            :src="form.image"
                            alt=""
                            class="h-11 w-11 border border-ink object-cover"
                        >
                        <label class="cursor-pointer border border-ink bg-canvas px-4 py-2.5 font-mono text-sm">
                            <input
                                type="file"
                                accept="image/png,image/jpeg,image/webp,image/gif"
                                class="hidden"
                                @change="onLauncherImage"
                            >
                            {{ t('appearance.launcher_image_choose') }}
                        </label>
                        <button
                            v-if="form.image"
                            type="button"
                            class="border border-grid px-4 py-2.5 font-mono text-sm hover:border-ink"
                            @click="clearLauncherImage"
                        >
                            {{ t('appearance.launcher_image_remove') }}
                        </button>
                    </div>
                    <span class="font-mono text-[11px] text-muted">{{ t('appearance.launcher_image_hint') }}</span>
                </div>
            </div>

            <p class="font-mono text-[11px] leading-relaxed text-muted">{{ t('appearance.parameters_hint') }}</p>

            <div class="flex items-center gap-4">
                <button
                    type="submit"
                    class="veda-hover bg-ink px-7 py-3.5 text-sm font-semibold text-canvas hover:bg-ink/80 disabled:opacity-40 disabled:hover:bg-ink"
                    :disabled="saving || !canSave"
                >
                    {{ t('common.save') }}
                </button>
                <p v-if="error" class="font-mono text-sm">{{ error }}</p>
                <p v-else-if="message" class="font-mono text-sm text-muted">{{ message }}</p>
            </div>
        </form>
    </div>
</template>
