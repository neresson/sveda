import { createApp, markRaw } from 'vue';
import '../../css/admin.css';
import { installI18n } from '../i18n';
import AdminApp from './AdminApp.vue';
import DashboardApp from './DashboardApp.vue';
import UsageApp from './UsageApp.vue';
import LoginApp from './LoginApp.vue';
import ModelsApp from './ModelsApp.vue';
import McpApp from './McpApp.vue';
import PromptsApp from './PromptsApp.vue';
import AppearanceApp from './AppearanceApp.vue';
import RuntimeApp from './RuntimeApp.vue';
import SecurityApp from './SecurityApp.vue';
import PoliciesApp from './PoliciesApp.vue';
import SetupApp from './SetupApp.vue';
import SourcesApp from './SourcesApp.vue';

const pages = {
    login: LoginApp,
    setup: SetupApp,
    dashboard: DashboardApp,
    usage: UsageApp,
    runtime: RuntimeApp,
    security: SecurityApp,
    policies: PoliciesApp,
    models: ModelsApp,
    mcp: McpApp,
    prompts: PromptsApp,
    appearance: AppearanceApp,
    sources: SourcesApp,
};

const AUTHENTICATED_PAGES = ['dashboard', 'usage', 'runtime', 'security', 'policies', 'models', 'mcp', 'prompts', 'appearance', 'sources'];

const el = document.getElementById('sveda-admin');

if (el) {
    const payload = window.SvedaAdmin ?? {
        page: el.dataset.page,
        csrf: el.dataset.csrf,
        saveUrl: el.dataset.saveUrl,
        logoutUrl: el.dataset.logoutUrl,
        action: el.dataset.action,
        error: el.dataset.error,
        settings: {},
    };

    const page = payload.page && pages[payload.page] ? payload.page : 'dashboard';

    const mount = async () => {
        if (!AUTHENTICATED_PAGES.includes(page)) {
            const app = createApp(pages[page], payload);
            installI18n(app);
            app.mount(el);

            return;
        }

        let chatPlugin = null;
        const { createAdminSveda, requestAdminSvedaSession } = await import('./createAdminSveda');
        const session = await requestAdminSvedaSession(payload.chat?.sessionUrl, payload.csrf);
        if (session) {
            chatPlugin = createAdminSveda({
                origin: session.origin,
                token: session.token,
                prefix: payload.chat?.prefix,
                protocolMode: payload.chat?.protocol,
                models: payload.chat?.models ?? payload.settings?.models,
                appearance: payload.settings?.appearance ?? {},
            });
        }

        const spaPages = Object.fromEntries(
            AUTHENTICATED_PAGES.map((id) => [id, markRaw(pages[id])]),
        );

        const app = createApp(AdminApp, {
            initial: payload,
            pages: spaPages,
            enableChat: Boolean(chatPlugin),
        });
        installI18n(app);
        if (chatPlugin) {
            app.use(chatPlugin);
        }
        app.mount(el);
    };

    void mount();
}
