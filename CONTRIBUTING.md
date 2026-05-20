# Contributing to Jito BAM Template

Thank you for choosing to contribute to the Jito Block Engine Aggregator Middleware (BAM) Sidecar template! We appreciate your efforts to make Solana MEV sidecar infrastructure faster, safer, and more reliable.

## Code of Conduct

We expect all contributors to adhere to standard professional open-source communication and collaboration patterns.

## Technical Architecture Overview

The Jito BAM Sidecar is designed for high-throughput, low-overhead transaction deduplication and Jito bundle assembly:

```
                  ┌─────────────────────────────────────┐
                  │          Market Maker Bot           │
                  └──────────────────┬──────────────────┘
                                     │ (JSON POST over HTTP/2)
                                     ▼
                  ┌─────────────────────────────────────┐
                  │        Axum Ingestion Engine        │
                  │  - Zero-Overhead Token Bucket R.L.  │
                  │  - Ed25519 Cryptographic Signatures │
                  └──────────────────┬──────────────────┘
                                     │ (Lock-free MPSC Queue)
                                     ▼
                  ┌─────────────────────────────────────┐
                  │      FCFS Aggregator Tick Loop      │
                  │  - 50ms Tick Batch Window           │
                  │  - Real-time Market Deduplication   │
                  └──────────────────┬──────────────────┘
                                     │ (Jito Bundle Assembly)
                                     ▼
                  ┌─────────────────────────────────────┐
                  │       Jito Block Engine RPC         │
                  └─────────────────────────────────────┘
```

### Core Design Rules

1. **Performance First**: The hot path (`/submit` ingestion and the `aggregator_loop`) must remain completely lock-free. Never introduce blocking code, mutexes, or trace logging inside the aggregation loop.
2. **Prometheus Metrics**: Always increment standard atomic counters in `src/metrics.rs` instead of utilizing high-latency tracing locks.
3. **Compiler Warnings**: Pull requests must compile with absolutely zero compiler warnings (`-D warnings`). Always run `cargo check` and `cargo test` before submitting changes.

## Development Setup

### System Prerequisites
- Rust 1.75+ (Stable toolchain)
- Docker & Docker-Compose (Optional, for containerized deployments)

### Checking and Formatting
Before proposing changes, ensure they conform to standards:

```bash
# Check format
cargo fmt --all -- --check

# Run linter
cargo clippy --all-targets --all-features -- -D warnings

# Execute test suite
cargo test --workspace --all-features
```

## Branch Strategy

- Core development occurs on standard feature branches (e.g., `feat/`, `fix/`).
- Merge requests should target `main` or `master`.
- Ensure a descriptive `CHANGELOG.md` update accompanies every feature addition.
