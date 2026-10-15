FROM rust:bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bin server

FROM debian:bookworm-slim
RUN groupadd --gid 10001 swarm && useradd --uid 10001 --gid swarm swarm && mkdir /data && chown swarm:swarm /data
COPY --from=build /build/target/release/server /usr/local/bin/server
COPY web /app/web
ENV LISTEN_ADDR=0.0.0.0:8080 DATABASE_PATH=/data/spareswarm.db WEB_DIR=/app/web
USER swarm
EXPOSE 8080
CMD ["server"]
