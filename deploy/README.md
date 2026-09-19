# Docker install

Pull the published image — no clone or `cargo build` required.

```bash
curl -fsSL https://raw.githubusercontent.com/neresson/sveda/main/deploy/compose.yaml -o compose.yaml
curl -fsSL https://raw.githubusercontent.com/neresson/sveda/main/deploy/compose.env -o .env
# Edit .env: DEEPSEEK_API_KEY, SVEDA_EMBED_HOST_API_KEY, SVEDA_CORS_ORIGINS
docker compose up -d
curl -s http://127.0.0.1:8787/sveda/ready
```

Human-friendly copies (browser): `https://sveda.dev/compose.yaml` — `curl` may get **403** if Cloudflare Bot Fight / challenge is on; use the GitHub raw URLs above for scripts and CI.

Image: `ghcr.io/neresson/sveda-server` (public after the first [Publish image](https://github.com/neresson/sveda/actions/workflows/publish-image.yml) run and GHCR package visibility is **Public**).

## Production pin (runtime.sveda.dev)

Do **not** rely on the floating `:latest` tag in production. Every push to `main` also tags `sha-<7-char>` on GHCR.

On the VPS, set in `.env` next to `compose.yaml`:

```bash
SVEDA_IMAGE_TAG=sha-522804e
```

`compose.yaml` uses `ghcr.io/neresson/sveda-server:${SVEDA_IMAGE_TAG:-latest}` with `pull_policy: if_not_present`. A new pin triggers `docker compose pull` on the next deploy.

## Auto-rollout from GitHub Actions

After [Publish image](https://github.com/neresson/sveda/actions/workflows/publish-image.yml) merges a build on `main`, the `rollout` job (when configured) SSHs to the host, updates `SVEDA_IMAGE_TAG` in `.env`, runs `docker compose pull` + `up -d`, waits for `GET /sveda/ready`, and checks that `GET /sveda/health` reports `revision` equal to the deployed git SHA.

Repository secrets (all required for rollout):

| Secret | Purpose |
|--------|---------|
| `RUNTIME_SSH_HOST` | VPS hostname |
| `RUNTIME_SSH_USER` | SSH user |
| `RUNTIME_SSH_KEY` | Private key |
| `RUNTIME_COMPOSE_DIR` | Directory containing `compose.yaml` and `.env` |

If secrets are missing, the rollout job is skipped; you deploy manually.

**Rollback:** Actions → Publish image → Run workflow → set `rollout_git_sha` to the full commit SHA you want (skips rebuild, pulls that `sha-*` tag only).

Host pages (sveda.dev) load `/build/sveda/sveda-chat.js` from the running sidecar. The landing worker adds `?v=<revision>` from `/sveda/health` so embed assets are not stuck in CDN cache after a roll.

Contributors building from source: use the repo root `docker compose up --build`.

First-time GHCR publish: [PUBLISH.md](PUBLISH.md).
