import { createVeda } from '@veda-ai/vue';
import { i18n } from '../i18n';

const chatEndpoints = (origin, prefix) => {
    const base = String(origin ?? '').replace(/\/$/, '');
    const path = String(prefix ?? 'veda').replace(/^\/+|\/+$/g, '') || 'veda';

    return {
        stream: `${base}/${path}/stream`,
        message: `${base}/${path}/message`,
        histories: `${base}/${path}/chat-histories`,
        documentsExtract: `${base}/${path}/documents/extract`,
    };
};

export const buildAdminVedaModels = (models) => {
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

export const requestAdminVedaSession = async (sessionUrl, csrf, fetchFn = fetch) => {
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
        if (!payload.origin || !payload.token) {
            return null;
        }

        return { origin: payload.origin, token: payload.token };
    } catch {
        return null;
    }
};

export const createAdminVeda = ({ origin, token, prefix, protocolMode, models } = {}) => {
    return createVeda({
        endpoints: chatEndpoints(origin, prefix),
        protocolMode: protocolMode === 'vercel' ? 'vercel' : 'veda',
        credentials: 'omit',
        headers: () => ({
            'X-Requested-With': 'XMLHttpRequest',
            'X-Veda-Embed-Token': token,
        }),
        locale: i18n.global.locale.value,
        brand: { name: 'Veda' },
        models: buildAdminVedaModels(models),
        quickPrompts: [],
    });
};
