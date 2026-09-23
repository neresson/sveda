# syntax=docker/dockerfile:1

FROM node:22-bookworm-slim AS ui
WORKDIR /src
RUN corepack enable && corepack prepare pnpm@9 --activate
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml tsconfig.base.json ./
COPY packages ./packages
COPY apps/runtime/package.json apps/runtime/package-lock.json ./apps/runtime/
RUN pnpm install --frozen-lockfile
WORKDIR /src/apps/runtime
RUN npm ci
COPY apps/runtime ./
RUN npm run build

FROM rust:1-bookworm AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --release -p sveda-server \
    && cp /src/target/release/sveda-server /tmp/sveda-server

FROM debian:bookworm-slim
ARG SVEDA_REVISION=dev
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 65532 --create-home sveda
COPY --from=rust /tmp/sveda-server /usr/local/bin/sveda-server
COPY --from=ui /src/apps/runtime/public/build /app/public/build
USER sveda
ENV SVEDA_BIND=0.0.0.0:8787 \
    SVEDA_ADMIN_DIST=/app/public/build \
    SVEDA_REVISION=$SVEDA_REVISION
EXPOSE 8787
HEALTHCHECK --interval=15s --timeout=3s --start-period=10s --retries=5 \
    CMD curl -fsS http://127.0.0.1:8787/sveda/ready >/dev/null
ENTRYPOINT ["sveda-server"]
