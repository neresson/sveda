import { applySvedaAppearance, createSveda } from '@sveda-ai/vue';
import { createApp } from 'vue';
import '../../css/admin.css';
import { buildAdminSvedaModels, createAdminSveda } from '../admin/createAdminSveda';
import { installI18n } from '../i18n';
import EmbedChat from './EmbedChat.vue';

const payload = window.SvedaEmbed ?? null;
const origin = payload?.origin || (typeof window === 'undefined' ? '' : window.location.origin);

if (!payload?.token || !origin) {
    console.error('Sveda embed: missing token or origin.');
} else {
    if (payload.appearance) {
        applySvedaAppearance(payload.appearance);
    }

    const plugin = createAdminSveda({
        origin,
        token: payload.token,
        prefix: payload.prefix,
        protocolMode: payload.protocol,
        models: buildAdminSvedaModels(payload.models),
        hostEmbed: true,
        hideLauncher: payload.hideLauncher !== false,
    });

    const app = createApp(EmbedChat, {
        brandName: payload.appearance?.brand?.name || 'Sveda',
        pageUrl: typeof window === 'undefined' ? '' : window.location.href,
    });

    installI18n(app);
    app.use(plugin);
    app.mount('#sveda-embed');
}
