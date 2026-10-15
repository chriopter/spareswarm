FROM rust:bookworm@sha256:114c7a4425406451c2866b6aafe69fe29b1b298832db1277d411ac73c82d04d6 AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock rust-toolchain.toml PROMPT.md ./
COPY cli ./cli
COPY server ./server
RUN cargo build --release --locked -p spareswarm-server

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
RUN apt-get update && apt-get install -y --no-install-recommends curl && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 swarm && useradd --uid 10001 --gid swarm swarm \
    && mkdir /data && chown swarm:swarm /data
COPY --from=build /build/target/release/spareswarm-server /usr/local/bin/server
COPY web /app/web
ENV LISTEN_ADDR=0.0.0.0:8080 DATABASE_PATH=/data/spareswarm.db WEB_DIR=/app/web
USER swarm
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD curl -fsS http://127.0.0.1:8080/health || exit 1
CMD ["server"]
