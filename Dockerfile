# Multi-stage build for Jito BAM Maker Quote Updater Sidecar
FROM rust:1.89-slim-bookworm AS builder

# Install system build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    git \
    g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Pre-compile dummy structure for efficient layer caching
RUN mkdir -p src/bin && \
    echo "pub mod plugin; pub mod bundler; pub mod zk; pub mod example_impl; pub mod nft_mint_impl; pub mod maker_plugin;" > src/lib.rs && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/generate_payload.rs && \
    mkdir -p src/plugin src/bundler src/zk src/example_impl src/nft_mint_impl src/maker_plugin && \
    echo "pub trait BamPlugin {}" > src/plugin.rs && \
    echo "pub struct JitoBundler;" > src/bundler.rs && \
    echo "pub struct ZkModule;" > src/zk.rs && \
    echo "pub struct ExampleHeartbeatPlugin;" > src/example_impl.rs && \
    echo "pub struct ExampleNftMintPlugin;" > src/nft_mint_impl.rs && \
    echo "pub struct MakerQuotePlugin;" > src/maker_plugin.rs && \
    cargo build --release && \
    rm -rf src/

# Copy real source code
COPY src ./src

# Build the real application and helper binaries
RUN cargo build --release

# Production stage
FROM debian:bookworm-slim AS runner

# Install runtime SSL certs and dynamic libraries
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binaries
COPY --from=builder /app/target/release/jito-bam-template /app/jito-bam-sidecar
COPY --from=builder /app/target/release/generate_payload /app/generate_payload

# Expose HTTP API Port
EXPOSE 3030

# Default environment variables
ENV RUST_LOG=info
ENV PORT=3030

# Run sidecar by default
CMD ["/app/jito-bam-sidecar"]
