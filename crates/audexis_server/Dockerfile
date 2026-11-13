FROM rust:1.99.1-slim-trixie AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked --package audexis_server \
    && install -Dm755 target/release/audexis_server /out/audexis_server

FROM debian:trixie-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 audexis \
    && useradd --uid 10001 --gid audexis --no-create-home --shell /usr/sbin/nologin audexis \
    && install -d -o audexis -g audexis /app/data /app/data/ingest /app/data/variant-input /app/data/media

COPY --from=builder /out/audexis_server /usr/local/bin/audexis_server
WORKDIR /app
ENV AUDEXIS_BIND=0.0.0.0:3000 \
    AUDEXIS_INGEST_DIR=/app/data/ingest \
    AUDEXIS_VARIANT_INPUT_DIR=/app/data/variant-input \
    AUDEXIS_FFMPEG_PATH=/usr/bin/ffmpeg

USER audexis
EXPOSE 3000
STOPSIGNAL SIGINT
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl --fail --silent --output /dev/null http://127.0.0.1:3000/health || exit 1
ENTRYPOINT ["/usr/local/bin/audexis_server"]
