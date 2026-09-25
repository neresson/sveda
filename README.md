# Sveda

Open-source embeddable AI agent: a JS client, host SDKs, and a self-hosted Rust sidecar.

- Site: [sveda.dev](https://sveda.dev)
- Docs: [sveda.dev/docs](https://sveda.dev/docs)

## Packages

| Package | Description |
|---------|-------------|
| `@sveda-ai/protocol` | Sveda Wire Protocol v1 — event schemas, constants, Vercel AI SDK adapter |
| `@sveda-ai/core` | Framework-agnostic client — streaming, tools, context registries |
| `@sveda-ai/chat` | Headless chat store, streaming, appearance, and shared helpers |
| [`@sveda-ai/vue`](https://sveda.dev/docs/chat-ui/vue) | Vue 3 SDK — `<SvedaChat>`, composables, default UI theme |
| [`@sveda-ai/react`](https://sveda.dev/docs/chat-ui/react) | React SDK — `<SvedaChat>`, hooks, default UI theme |
| [`@sveda-ai/svelte`](https://sveda.dev/docs/chat-ui/svelte) | Svelte 5 SDK — `<SvedaChat>`, stores, default UI theme |
| [`@sveda-ai/solid`](https://sveda.dev/docs/chat-ui/solid) | Solid SDK — `<SvedaChat>`, signals, default UI theme |
| `@sveda-ai/element` | Web component `<sveda-chat>` for script-tag embedding |
| [`sveda-ai/php-sdk`](https://github.com/neresson/sveda-php-sdk) | PHP SDK — sidecar HTTP API (embed tokens, streaming, histories) |
| [`sveda-ai/laravel-sdk`](https://github.com/neresson/sveda-laravel-sdk) | Laravel host SDK — HTTP client + automatic MCP server |
| [`@sveda-ai/node-sdk`](https://github.com/neresson/sveda-node-sdk) | Node SDK — sidecar HTTP API |
| [`sveda-python-sdk`](https://github.com/neresson/sveda-python-sdk) | Python SDK — sidecar HTTP API |
| [`sveda-ruby-sdk`](https://github.com/neresson/sveda-ruby-sdk) | Ruby SDK — sidecar HTTP API |
| [`sveda-go-sdk`](https://github.com/neresson/sveda-go-sdk) | Go SDK — sidecar HTTP API |
| [`sveda-java-sdk`](https://github.com/neresson/sveda-java-sdk) | Java SDK — sidecar HTTP API |
| [`sveda-dotnet-sdk`](https://github.com/neresson/sveda-dotnet-sdk) | .NET SDK — sidecar HTTP API |
| `sveda-server` | Rust sidecar — agent, providers, embed HTTP, storage. Settings come from `sveda.yaml` and env; `/admin/settings` is the machine API |

## Development

```bash
pnpm install
pnpm test
pnpm build
cargo test --workspace
```

Cluster integration tests run when `SVEDA_TEST_DATABASE_URL` and `SVEDA_TEST_REDIS_URL` are set (CI provides them; locally `docker compose up -d postgres redis`, then `postgres://sveda:sveda@127.0.0.1:5433/sveda` and `redis://127.0.0.1:6380`).

## Install (Docker)

No clone required — pull the published image:

```bash
curl -fsSL https://raw.githubusercontent.com/neresson/sveda/main/deploy/compose.yaml -o compose.yaml
curl -fsSL https://raw.githubusercontent.com/neresson/sveda/main/deploy/compose.env -o .env
# Edit .env: DEEPSEEK_API_KEY, SVEDA_EMBED_HOST_API_KEY, SVEDA_CORS_ORIGINS, SVEDA_ADMIN_API_KEY
# Optional: mount deploy/sveda.yaml and set SVEDA_CONFIG=/etc/sveda/sveda.yaml
# That file holds prompts, models, embeddings, mcp.mcpServers, policies, appearance, cors, web, and security.
# Reload that file with kill -HUP after edits. There is no admin UI in the sidecar.
docker compose up -d
curl -s http://127.0.0.1:8787/sveda/ready
```

See [deploy/README.md](deploy/README.md). Image: `ghcr.io/neresson/sveda-server:latest` (set the GHCR package to Public after the first [Publish image](https://github.com/neresson/sveda/actions/workflows/publish-image.yml) run).

## Develop (local stack)

From this checkout, build and run Postgres + Redis + `sveda-server` (exposes store ports for tests):

```bash
docker compose up --build
```

Health: `GET /sveda/health`. Readiness (store ping): `GET /sveda/ready`.

Kubernetes: `charts/sveda-server` — use `--set image.tag=latest` until a matching `v*` image tag exists.

## License

Apache License 2.0. See [LICENSE](LICENSE).

Host SDKs live in sibling repositories and use the same license. Sveda Cloud is a separate proprietary control plane.
