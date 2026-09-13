# Veda AI

Open-source embeddable AI copilot framework with a universal wire protocol, framework-agnostic JavaScript core, Vue SDK, web component, and Laravel runtime.

## Packages

| Package | Description |
|---------|-------------|
| `@veda-ai/protocol` | Veda Wire Protocol v1 — event schemas, constants, Vercel AI SDK adapter |
| `@veda-ai/core` | Framework-agnostic client — streaming, tools, context registries |
| `@veda-ai/vue` | Vue 3 SDK — `<VedaChat>`, composables, default UI theme |
| `@veda-ai/element` | Web component `<veda-chat>` for script-tag embedding |
| `veda-ai/laravel` | Laravel runtime — agent, providers, Tool/Context API, storage |

## Development

```bash
pnpm install
pnpm test
pnpm build
```

## License

MIT
