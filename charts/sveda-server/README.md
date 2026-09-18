# sveda-server Helm chart

Installs horizontally scalable `sveda-server` replicas. Chat history and admin settings live in Postgres. Occupancy, throttle, MCP creds, and settings revision live in Redis.

## Production

Point the chart at managed stores:

```bash
helm install sveda charts/sveda-server \
  --set image.tag=latest \
  --set secrets.appKey="$SVEDA_APP_KEY" \
  --set secrets.adminApiKey="$SVEDA_ADMIN_API_KEY" \
  --set secrets.hostApiKey="$SVEDA_EMBED_HOST_API_KEY" \
  --set databaseUrl="$SVEDA_DATABASE_URL" \
  --set redisUrl="$SVEDA_REDIS_URL"
```

## Local cluster (kind / k3d)

Bundled Postgres and Redis are **demo only** (single replica, official images, not HA):

```bash
helm install sveda charts/sveda-server \
  --set image.tag=latest \
  --set postgresql.enabled=true \
  --set redis.enabled=true \
  --set secrets.appKey=dev-app-key-change-me \
  --set secrets.adminApiKey=dev-admin-key-change-me \
  --set secrets.hostApiKey=dev-host-key-change-me
```

Probes: `GET /sveda/health` (live), `GET /sveda/ready` (Postgres + Redis).
