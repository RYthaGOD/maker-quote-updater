# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-05-20

### Added
- **High-Throughput Observability & Metrics Layer**: 
  - Lock-free global Prometheus metrics registry `src/metrics.rs` using standard atomic values (`AtomicU64`).
  - Served Prometheus `/metrics` exposition route dynamically.
  - Tracking queue depth, drop ratios, transaction counts, and bundle submission latency sums.
- **Boot-time Validation**: Fast check for cryptographic base58 inputs (`BAM_AUTHORITY_KEY`, `MAKER_PUBKEY`, `ALLOWED_MARKETS`) on startup.
- **Graceful Shutdown**: Intercepting UNIX signals (`SIGTERM`, `ctrl_c`) to safely complete transaction aggregation before terminating Axum.
- **High-Speed Rate Limiting**: Embedded custom thread-safe Token-Bucket rate limiter on `/submit` to guard CPU cycles.
- **OpenAPI / Swagger Integration**: Dynamic `/openapi.json` route serving the API schema.
- **Continuous Integration Pipeline**: Added `.github/workflows/ci.yml` running formatters, clippy lints, and test checks automatically on push.
- **Testing Suite**: Added `tests/integration_test.rs` compiling and passing 100% warning-free, validating signature verification, allowlist logic, and metrics formatting.
- **Production Containerization**: Multi-stage `Dockerfile`, `docker-compose.yml`, and `.dockerignore`.
- **DX Upgrades**:
  - `examples/client_example.ts` showcasing the exact client-side Ed25519 signing flow in TypeScript.
  - Declared explicit binaries in `Cargo.toml` (`jito-bam-template`, `generate_payload`).

### Changed
- Standardized file layouts to comply with 100% clean warnings-free Rust compilation.
