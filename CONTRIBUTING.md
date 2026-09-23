# Contributing

1. Run `pnpm install` at the monorepo root.
2. Run `pnpm test`, `pnpm typecheck`, and `pnpm lint` before submitting changes.
3. For Rust changes, run `cargo fmt --all`, `cargo clippy --workspace --all-targets`, and `cargo test --workspace`.
4. For sidecar protocol changes, update `packages/protocol/contracts/sidecar.v1.json`, run package tests, then `bash scripts/sync-sidecar-contract.sh` in sibling SDK repos.
5. Optional: `docker compose up -d postgres redis` and export `SVEDA_TEST_DATABASE_URL=postgres://sveda:sveda@127.0.0.1:5433/sveda` plus `SVEDA_TEST_REDIS_URL=redis://127.0.0.1:6380` so cluster store tests run.
6. E2E tiers: default Playwright run is **hermetic** (in-memory store). Set `SVEDA_E2E_PERSISTENT=1` for Postgres/Redis (`pnpm --dir e2e run test:e2e:persistent`). Live DeepSeek specs run when `DEEPSEEK_API_KEY` is set.
7. Use changesets for user-facing npm package changes.

## Release checklist

1. Land changes on `main` with green CI.
2. Add a changeset when npm packages change; bump Rust/npm versions as needed.
3. Tag `v*` on `main` for server releases.
4. Wait for **SDK compatibility** (`sdk-compat.yml`) and image publish workflows to finish.
5. Verify [`COMPATIBILITY.md`](COMPATIBILITY.md) and roll out runtime when SSH secrets are configured.
6. Publish SDK tags independently; each SDK publish can trigger `repository_dispatch` to re-run the matrix when `SVEDA_REPO_DISPATCH_TOKEN` is configured.
