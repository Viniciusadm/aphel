# syntax=docker/dockerfile:1

FROM rust:1.99.0-bookworm AS modules
WORKDIR /src
COPY rust-toolchain.toml ./
COPY Makefile ./
COPY back/ back/
COPY web/ web/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/back/target \
    make modules

FROM node:24-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
COPY --from=modules /src/web/src/modules.generated.ts ./src/modules.generated.ts
COPY --from=modules /src/web/modules.aliases.json /src/web/tsconfig.modules.json ./
RUN npm run build

FROM rust:1.99.0-bookworm AS back
WORKDIR /src/back
COPY rust-toolchain.toml /src/rust-toolchain.toml
ENV SQLX_OFFLINE=true
COPY back/ ./
COPY --from=modules /src/back/app/Cargo.toml ./app/Cargo.toml
COPY --from=modules /src/back/app/src/modules.rs ./app/src/modules.rs
COPY catalog/ /src/catalog/
COPY --from=web /src/web/dist /src/web/dist
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/back/target \
    cargo build --release -p aphel --bin aphel \
    && cp target/release/aphel /usr/local/bin/aphel

FROM scratch AS binary
COPY --from=back /usr/local/bin/aphel /aphel

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 aphel \
    && useradd --system --uid 10001 --gid aphel --home-dir /app --shell /usr/sbin/nologin aphel \
    && mkdir -p /app/storage \
    && chown -R aphel:aphel /app
WORKDIR /app
COPY --from=back /usr/local/bin/aphel /usr/local/bin/aphel
ENV HTTP_HOST=0.0.0.0 \
    HTTP_PORT=3001 \
    RUST_LOG=info
USER 10001:10001
EXPOSE 3001
VOLUME ["/app/storage"]
ENTRYPOINT ["/usr/local/bin/aphel"]
CMD ["serve"]
