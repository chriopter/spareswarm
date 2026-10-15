FROM rust:1.99.0-trixie@sha256:81015672d6939b0265f09adf96d5de125482bb1330106076956b0518e74edc90 AS build
WORKDIR /build
COPY PROMPT.md ./
COPY server ./server
WORKDIR /build/server
ARG BUILD_COMMIT=development
ENV BUILD_COMMIT=$BUILD_COMMIT
RUN cargo build --release --locked

FROM debian:trixie-slim@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f
RUN apt-get update && apt-get upgrade -y && apt-get install -y --no-install-recommends curl ca-certificates && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 swarm && useradd --uid 10001 --gid swarm swarm \
    && mkdir /data && chown swarm:swarm /data
COPY --from=build /build/server/target/release/spareswarm-server /usr/local/bin/server
COPY web /app/web
COPY projects /app/projects
ENV LISTEN_ADDR=0.0.0.0:8080 DATABASE_PATH=/data/spareswarm.db WEB_DIR=/app/web PROJECTS_DIR=/app/projects
ARG BUILD_COMMIT=development
LABEL org.opencontainers.image.source="https://github.com/chriopter/spareswarm" \
      org.opencontainers.image.revision=$BUILD_COMMIT
USER swarm
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD curl -fsS http://127.0.0.1:8080/health || exit 1
CMD ["server"]
