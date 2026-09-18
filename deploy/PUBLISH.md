# First-time image publish checklist

After merging Docker publish changes to `main`:

1. **Run the workflow** — [Actions → Publish image → Run workflow](https://github.com/neresson/sveda/actions/workflows/publish-image.yml), or push to `main`.
2. **Make the package public** — GitHub → your profile → Packages → `sveda-server` → Package settings → Change visibility → **Public**. Without this, `docker pull` requires `docker login ghcr.io` even when the repo is public.
3. **Verify pull** (no login):
   ```bash
   docker pull ghcr.io/neresson/sveda-server:latest
   ```
4. **Verify install**:
   ```bash
   curl -fsSL https://raw.githubusercontent.com/neresson/sveda/main/deploy/compose.yaml -o compose.yaml
   curl -fsSL https://raw.githubusercontent.com/neresson/sveda/main/deploy/compose.env -o .env
   docker compose up -d
   curl -s http://127.0.0.1:8787/sveda/ready
   ```

Optional Docker Hub mirror: add a separate workflow job that runs only when repository variable `PUBLISH_DOCKERHUB` is `true` and Hub secrets are configured (GitHub does not allow `secrets` in `if:` expressions).
