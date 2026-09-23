import { defineConfig, devices } from '@playwright/test';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { applyLiveE2eEnv } from './tests/helpers/env';

const outboundProxy = {
  HTTP_PROXY: process.env.HTTP_PROXY,
  HTTPS_PROXY: process.env.HTTPS_PROXY,
  http_proxy: process.env.http_proxy,
  https_proxy: process.env.https_proxy,
  ALL_PROXY: process.env.ALL_PROXY,
  all_proxy: process.env.all_proxy,
};

for (const key of ['HTTP_PROXY', 'HTTPS_PROXY', 'http_proxy', 'https_proxy', 'ALL_PROXY', 'all_proxy']) {
  delete process.env[key];
}
process.env.NO_PROXY = '127.0.0.1,localhost,::1';
process.env.no_proxy = process.env.NO_PROXY;
applyLiveE2eEnv();

const root = dirname(fileURLToPath(import.meta.url));
const serveScript = process.env.SVEDA_E2E_PERSISTENT === '1' ? 'serve-persistent.sh' : 'serve.sh';
const origin = process.env.SVEDA_E2E_ORIGIN ?? 'http://127.0.0.1:18787';
const localNoProxy = '127.0.0.1,localhost,::1';

const serverEnv = {
  ...process.env,
  SVEDA_ENV_FILE: '/dev/null',
  SVEDA_BIND: '127.0.0.1:18787',
  SVEDA_EMBED_ENABLED: 'true',
  SVEDA_ADMIN_API_KEY: process.env.SVEDA_ADMIN_API_KEY ?? 'sveda-e2e-admin-key',
  SVEDA_APP_KEY: process.env.SVEDA_APP_KEY ?? 'sveda-e2e-app-key',
  SVEDA_DATABASE_URL: '',
  SVEDA_REDIS_URL: '',
  NO_PROXY: localNoProxy,
  no_proxy: localNoProxy,
  ...(outboundProxy.HTTP_PROXY ? { HTTP_PROXY: outboundProxy.HTTP_PROXY } : {}),
  ...(outboundProxy.HTTPS_PROXY ? { HTTPS_PROXY: outboundProxy.HTTPS_PROXY } : {}),
  ...(outboundProxy.http_proxy ? { http_proxy: outboundProxy.http_proxy } : {}),
  ...(outboundProxy.https_proxy ? { https_proxy: outboundProxy.https_proxy } : {}),
  ...(outboundProxy.ALL_PROXY ? { ALL_PROXY: outboundProxy.ALL_PROXY } : {}),
  ...(outboundProxy.all_proxy ? { all_proxy: outboundProxy.all_proxy } : {}),
};

export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['github'], ['list']] : 'list',
  use: {
    ...devices['Desktop Chrome'],
    baseURL: origin,
    viewport: { width: 1280, height: 800 },
    trace: 'retain-on-failure',
    proxy: undefined,
  },
  webServer: process.env.SVEDA_E2E_ORIGIN
    ? undefined
    : {
        command: `bash ${serveScript}`,
        url: `${origin}/sveda/health`,
        reuseExistingServer: !process.env.CI,
        timeout: 180_000,
        stdout: 'pipe',
        stderr: 'pipe',
        cwd: root,
        env: serverEnv,
      },
});
