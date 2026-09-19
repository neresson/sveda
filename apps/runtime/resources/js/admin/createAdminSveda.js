import { createSveda } from '@sveda-ai/vue';
import { i18n } from '../i18n';

const chatEndpoints = (origin, prefix) => {
    const base = String(origin ?? '').replace(/\/$/, '');
    const path = String(prefix ?? 'sveda').replace(/^\/+|\/+$/g, '') || 'sveda';

    return {
        stream: `${base}/${path}/stream`,
        message: `${base}/${path}/message`,
        histories: `${base}/${path}/chat-histories`,
        documentsExtract: `${base}/${path}/documents/extract`,
    };
};

export const buildAdminSvedaModels = (models) => {
    if (!Array.isArray(models)) {
        return [];
    }

    return models
        .map((model) => {
            const id = String(model?.id ?? '').trim();
            if (id === '') {
                return null;
            }

            return {
                id,
                label: String(model?.label ?? id),
                supportsThinking: Boolean(model?.supportsThinking ?? model?.supports_thinking ?? model?.thinking),
            };
        })
        .filter(Boolean);
};

export const requestAdminSvedaSession = async (sessionUrl, csrf, fetchFn = fetch) => {
    if (!sessionUrl) {
        return null;
    }

    const headers = {
        Accept: 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
    };

    if (csrf) {
        headers['X-CSRF-TOKEN'] = csrf;
    }

    try {
        const response = await fetchFn(sessionUrl, {
            method: 'POST',
            credentials: 'same-origin',
            headers,
        });

        if (!response.ok) {
            return null;
        }

        const payload = await response.json();
        const origin = String(payload.origin ?? '').trim() || (typeof window === 'undefined' ? '' : window.location.origin);
        if (!origin || !payload.token) {
            return null;
        }

        return { origin, token: payload.token };
    } catch {
        return null;
    }
};

export const createAdminSveda = ({ origin, token, prefix, protocolMode, models, appearance, hostEmbed, hideLauncher } = {}) => {
    return createSveda({
        endpoints: chatEndpoints(origin, prefix),
        protocolMode: protocolMode === 'vercel' ? 'vercel' : 'sveda',
        credentials: 'omit',
        hostEmbed: Boolean(hostEmbed),
        hideLauncher: Boolean(hideLauncher),
        headers: () => ({
            'X-Requested-With': 'XMLHttpRequest',
            'X-Sveda-Embed-Token': token,
        }),
        locale: i18n.global.locale.value,
        brand: { name: 'Sveda' },
        models: buildAdminSvedaModels(models),
        appearance: appearance && typeof appearance === 'object' ? appearance : {},
        quickPrompts: [],
    });
};
