import { createApp } from 'vue';
import '../../css/admin.css';
import { installI18n } from '../i18n';
import LoginApp from './LoginApp.vue';
import ModelsApp from './ModelsApp.vue';
import SetupApp from './SetupApp.vue';

const pages = {
    login: LoginApp,
    setup: SetupApp,
    models: ModelsApp,
};

const el = document.getElementById('veda-admin');

if (el) {
    const payload = window.VedaAdmin ?? {
        page: el.dataset.page,
        csrf: el.dataset.csrf,
        saveUrl: el.dataset.saveUrl,
        logoutUrl: el.dataset.logoutUrl,
        action: el.dataset.action,
        error: el.dataset.error,
        settings: {},
    };

    const page = payload.page && pages[payload.page] ? payload.page : 'models';
    const app = createApp(pages[page], payload);
    installI18n(app);
    app.mount(el);
}
