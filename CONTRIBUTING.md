# Contributing

1. Run `pnpm install` at the monorepo root.
2. Run `pnpm test` and `pnpm typecheck` before submitting changes.
3. For sidecar changes, run `cargo test --workspace`.
4. Optional: `docker compose up -d postgres redis` and export `SVEDA_TEST_DATABASE_URL=postgres://sveda:sveda@127.0.0.1:5433/sveda` plus `SVEDA_TEST_REDIS_URL=redis://127.0.0.1:6380` so cluster store tests run.
5. Use changesets for user-facing package changes.
