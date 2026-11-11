# Imagen de la web: el servidor axum (crates/server) sirve la interfaz de React ya compilada.
# Se publica en GHCR desde .github/workflows/image.yml. Despliegue: docs/DEPLOY.md.

FROM node:24-slim AS ui
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@10 --activate
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY index.html tsconfig.json vite.config.ts ./
COPY src ./src
RUN pnpm build

FROM rust:1-bookworm AS server
RUN apt-get update && apt-get install -y --no-install-recommends cmake clang && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# El workspace declara src-tauri como miembro: su manifiesto tiene que estar, aunque no se compile.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY src-tauri ./src-tauri
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked -p lolup-server && cp target/release/lolup-server /usr/local/bin/

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=server /usr/local/bin/lolup-server /usr/local/bin/lolup-server
COPY --from=ui /app/dist ./dist
ENV LOLUP_ADDR=0.0.0.0:3000 \
    LOLUP_DB=/data/lolup.db
VOLUME /data
EXPOSE 3000
CMD ["lolup-server"]
