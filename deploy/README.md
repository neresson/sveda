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

Image: `ghcr.io/neresson/sveda-server:latest` (public after the first [Publish image](https://github.com/neresson/sveda/actions/workflows/publish-image.yml) run and GHCR package visibility is **Public**).

`compose.yaml` sets `pull_policy: always` on `sveda-server`. That pulls a new `:latest` when **you** run `docker compose up -d`. It does not watch the registry by itself.

## Auto-update after Publish image

Push to `main` builds `ghcr.io/neresson/sveda-server:latest`. Host pages (including sveda.dev) load `/build/sveda/sveda-chat.js` from the **running** sidecar, not from the GitHub repo. Pick one of:

**Watchtower on the host** (no GitHub secrets):

```bash
docker compose -f compose.yaml -f compose.watchtower.yaml up -d
```

Watchtower checks GHCR about once a minute and recreates `sveda-server` when the digest changes.

**GitHub Actions SSH** (push → pull → restart): repo secrets `RUNTIME_SSH_HOST`, `RUNTIME_SSH_USER`, `RUNTIME_SSH_KEY`, `RUNTIME_COMPOSE_DIR`. Optional CDN purge: secret `CLOUDFLARE_CACHE_TOKEN` (Cache Purge), `CLOUDFLARE_ZONE_ID`, and repository variable `RUNTIME_EMBED_ORIGIN` (e.g. `https://runtime.sveda.dev`). If those are empty, the rollout job no-ops.

Kubernetes: the chart defaults to `image.pullPolicy=Always` for floating tags. A new `:latest` still needs a rollout (`kubectl rollout restart`) unless a controller restarts pods.

Embed JS/CSS are served with `Cache-Control: max-age=0, must-revalidate` so Cloudflare revalidates after the container rolls. Until that image is running, purge `/build/sveda/*` once.

Contributors building from source: use the repo root `docker compose up --build`.

First-time GHCR publish: [PUBLISH.md](PUBLISH.md).
