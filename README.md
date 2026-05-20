# Jito Maker Quote Updater Sidecar

This sidecar is a high-frequency transaction aggregator and deduplicator for Solana market makers. It acts as an intermediate queue between a trading bot and the Jito Block Engine.

## What It Does

When quoting on Solana orderbooks or AMMs, network jitter and slot scheduling randomness make it difficult to guarantee exactly when a transaction executes. Market makers typically spam quote updates to ensure at least one transaction lands, which increases network congestion, transaction fee waste, and the risk of execution at stale prices.

This sidecar addresses those challenges:
1. **Ingestion**: It exposes a high-throughput HTTP POST endpoint (`/submit`) where trading bots submit quote updates.
2. **Deduplication**: Incoming quote updates are held in memory. During each batching window (default: 50 milliseconds), only the newest quote update for each market ID is kept; all older, unexecuted quote updates for that market are discarded.
3. **Replay Protection**: Every payload includes a Unix timestamp in milliseconds. The sidecar rejects any payload whose timestamp is older than 5 seconds to prevent historical signature replay attacks.
4. **Bundle Submission**: Every 50ms, the sidecar packs all active, deduplicated updates into a Jito Bundle and submits it directly to the Jito Block Engine.

---

## Technical Mechanics & Data Flow

```
+------------------+             +--------------------+             +------------------+
| Market Maker Bot | --(HTTP)--> | Axum Sidecar Queue | --(Bundle)-> | Jito Block Engine|
+------------------+             +--------------------+             +------------------+
                                   - Verify Signature
                                   - Check Timestamp
                                   - Keep Newest Quote
```

### 1. Cryptographic Payload Structure
Trading bots must serialize the quote updates and sign them using Ed25519. The sidecar deserializes the binary payload using `bincode` layout:

1. **market_id**: String length prefix (u64, little-endian) followed by UTF-8 bytes.
2. **bid_price**: u64 (little-endian).
3. **ask_price**: u64 (little-endian).
4. **size**: u64 (little-endian).
5. **timestamp_ms**: u64 (little-endian).

The sidecar verifies the signature against the pre-configured `MAKER_PUBKEY` and validates that the `timestamp_ms` is within 5 seconds of the sidecar's current system time.

### 2. Aggregator Loop
An asynchronous loop runs on a strict `tokio::time::interval` timer (50ms). It locks the incoming queue, groups the transactions by their market IDs, takes the newest one, constructs transaction instructions using the configured DEX program ID, signs the transactions using the `BAM_AUTHORITY_KEY`, appends a Jito tip transaction, and sends the bundle.

---

## Alignment with Jito and JTX

This sidecar integrates directly with Jito's Block Engine and conforms to JTX (Jito Transaction Execution) principles:

* **Mempool Bypass**: Transactions are sent directly to Jito Block Engine endpoints via the bundle submission RPC, completely bypassing the public Solana mempool.
* **Top-of-Block Execution**: Jito bundles are executed at the absolute beginning of a slot. This guarantees that quotes land exactly when intended.
* **Zero Network Spam**: Instead of spamming dozens of transactions per second to guarantee landing, a market maker sends updates to this local sidecar. The sidecar outputs exactly one deduplicated transaction per market per batch window. This reduces RPC load and network congestion.
* **Atomicity**: The transactions are sent as an atomic bundle. If any transaction in the bundle fails, the entire bundle is discarded, preventing partial or toxic quote executions.

---

## API Routes & Observability

The sidecar exposes a high-performance HTTP server built on Axum:

*   **`POST /submit`**: Ingestion endpoint for signed quote updates.
*   **`GET /openapi.json`**: Exposes the complete OpenAPI v3 schema for easy API integration and client generation.
*   **`GET /metrics`**: Serves live, zero-overhead Prometheus metrics. Metrics are updated atomically (`AtomicU64`) on the hot path with lock-free synchronization. Exposed metrics include:
    *   `jito_bam_incoming_requests_total`: Total quote update requests received.
    *   `jito_bam_deduped_updates_total`: Total stale quote updates dropped in memory.
    *   `jito_bam_bundle_submissions_total`: Cumulative bundle submissions categorized by status (`success` or `failure`).
    *   `jito_bam_queue_depth`: Current size of the in-memory aggregation channel.
    *   `jito_bam_processing_latency_avg_ms`: Live average of aggregation and block engine transmission latency.

---

## Rate Limiting & Resilience

To prevent resource exhaustion and protect downstream validator connections from quote spam, the sidecar implements an in-memory, thread-safe Token Bucket rate limiter.

Configure the rate limiter in `.env`:
*   **`RATE_LIMIT_MAX_BURST`**: Maximum token bucket capacity/burst requests allowed (Default: `100.0`).
*   **`RATE_LIMIT_REFILL_RATE`**: Rate at which tokens are refilled per second (Default: `50.0`).

---

## Graceful Shutdown

The sidecar is designed for high-availability cloud deployments. It intercepts host signals (`SIGINT` and `SIGTERM`). 

Upon signal interception:
1. The Axum HTTP server stops accepting new connections.
2. In-flight requests are drained and processed.
3. The aggregator loop drains remaining queue items, finalized transactions are safely processed, and the application exits cleanly with zero orphaned states.

---

## Configuration Setup

Create a `.env` file in the root directory:

```env
# Base58 private key of the BAM Authority paying for Jito tips and bundle fees
BAM_AUTHORITY_KEY=YOUR_BASE58_PRIVATE_KEY

# Base58 public key of the market making bot (for verifying incoming payload signatures)
MAKER_PUBKEY=YOUR_BOT_PUBLIC_KEY

# Comma-separated allowlist of market IDs (to prevent unauthorized queue allocation)
ALLOWED_MARKETS=SOL/USDC,BTC/USDC

# Aggregation interval in milliseconds
BAM_TICK_RATE_MS=50

# Server Port
PORT=3030

# Rate Limiting Configurations
RATE_LIMIT_MAX_BURST=100.0
RATE_LIMIT_REFILL_RATE=50.0

# Solana RPC and Jito Block Engine URLs
RPC_URL=https://api.mainnet-beta.solana.com
JITO_URL=https://mainnet.block-engine.jito.wtf/api/v1/bundles
```

---

## Execution

### Run Locally
```bash
# Verify compilation and format
cargo check

# Run tests (verifies signature, allowlist, and replay window logic)
cargo test -p jito-bam-template --test integration_test

# Start the sidecar
cargo run --release
```

### Test Payload Generator
Run the built-in generator to print a valid public key and output a ready-to-use signed testing curl command:
```bash
cargo run --bin generate_payload
```

### Run inside Docker
```bash
docker-compose up -d --build
```
