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

Contributors building from source: use the repo root `docker compose up --build`.

First-time GHCR publish: [PUBLISH.md](PUBLISH.md).
