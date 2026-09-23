# SDK compatibility matrix

This file is updated by the [`SDK compatibility`](.github/workflows/sdk-compat.yml) workflow after release tags.

| SDK | Status |
| --- | --- |
| php | pending |
| python | pending |
| node | pending |
| go | pending |
| java | pending |
| dotnet | pending |
| ruby | pending |
| laravel | pending |
| android | pending |

## Version policy

- **Sveda server** tags (`v*`) must pass the SDK compatibility matrix before image rollout on tag releases.
- **SDK** repos vendor [`packages/protocol/contracts/sidecar.v1.json`](packages/protocol/contracts/sidecar.v1.json) and run contract tests on every PR.
- SDK minor versions are independent; compatibility is defined by the sidecar contract `version` field (`1.0` today), not by matching semver across repos.

## Local matrix

```bash
cd sveda
cargo build -p sveda-server
bash scripts/sdk-compat/serve.sh &
export SVEDA_BASE_URL=http://127.0.0.1:18787
export SVEDA_HOST_KEY=sveda-compat-host-key
bash scripts/sdk-compat/run-sdk.sh php
```

Sync vendored contracts after protocol changes:

```bash
bash scripts/sync-sidecar-contract.sh
```
