# ============================================================================
# MITOSIS Orchestrator — Multistage Container Build
# ============================================================================

# Stage 1: Rust Builder
FROM rust:1.97-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Add wasm targets
RUN rustup target add wasm32-unknown-unknown

# Copy repository source code
COPY . .

# Compile WASM plugins
RUN RUSTFLAGS="-C link-arg=--import-undefined" cargo build -p d4games --target wasm32-unknown-unknown --release && \
    mkdir -p plugins && \
    cp target/wasm32-unknown-unknown/release/d4games.wasm plugins/

# Build release binary
RUN cargo build --bin mitosis-orchestrator --release

# Stage 2: Minimal Runtime
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 mitosis

# Copy binary, plugins, web dashboard, sample configs
COPY --from=builder /build/target/release/mitosis-orchestrator /app/mitosis-orchestrator
COPY --from=builder /build/plugins /app/plugins
COPY --from=builder /build/skins /app/skins
COPY --from=builder /build/web /app/web
COPY --from=builder /build/config.toml /app/config.toml

RUN chown -R mitosis:mitosis /app

USER mitosis

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/api/v1/status || exit 1

ENTRYPOINT ["/app/mitosis-orchestrator"]
