# Docker install

Pull the published image — no clone or `cargo build` required.

```bash
curl -fsSL https://sveda.dev/compose.yaml -o compose.yaml
curl -fsSL https://sveda.dev/compose.env -o .env
# Edit .env: DEEPSEEK_API_KEY, SVEDA_EMBED_HOST_API_KEY, SVEDA_CORS_ORIGINS
docker compose up -d
curl -s http://127.0.0.1:8787/sveda/ready
```

Alternate compose URL: `https://raw.githubusercontent.com/neresson/sveda/main/deploy/compose.yaml`

Image: `ghcr.io/neresson/sveda-server:latest` (public after the first [Publish image](https://github.com/neresson/sveda/actions/workflows/publish-image.yml) run and GHCR package visibility is **Public**).

Contributors building from source: use the repo root `docker compose up --build`.

First-time GHCR publish: [PUBLISH.md](PUBLISH.md).
