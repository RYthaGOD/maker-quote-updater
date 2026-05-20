# 🤖 Agent Intelligence Layer (BAM Template)

Welcome, Agent. This document is a high-density technical map designed to help you understand, extend, and debug this repository without a full-repo scan.

---

## 🏗️ Architectural Pattern: Aggregator-Bundler

This repo implements a high-performance sidecar for Jito BAM. 

### Core Intelligence Loop:
1.  **Ingestion**: `src/main.rs`: Axum server accepts JSON payloads and pipes them to an MPSC channel.
2.  **Aggregation**: `src/main.rs`: An async worker batches payloads based on `plugin.batch_size()` or a 5-second interval.
3.  **Strategy**: `src/plugin.rs`: The `BamPlugin` trait is the core extension point. It defines how to `verify()` and `build_instructions()` for a specific payload.
4.  **Bundling**: `src/bundler.rs`: Interfaces with Jito SDK to fetch tip accounts, construct transactions, and monitor bundle status.
5.  **State Proofs**: `src/zk.rs`: Optional module for resolving ZK-Compressed account state via Light Protocol.

---

## 🗺️ Component Heatmap

| Component | File | Role | Criticality |
| :--- | :--- | :--- | :--- |
| **Traits** | `src/plugin.rs` | Protocol definition for plugins. | High |
| **MEV Engine** | `src/bundler.rs` | Jito Block Engine & Tip logic. | Critical |
| **API/State** | `src/main.rs` | App state, routes, and aggregator. | High |
| **Example** | `src/example_impl.rs` | Concrete implementation reference. | Low |
| **ZK Module** | `src/zk.rs` | ZK-Compression state resolution. | Medium |

---

## 🛠️ How to Extend (Machine Recipe)

To implement a new strategy (e.g., Oracle Updates, Liquidations):

1.  **Define Payload**: Create a serializable struct in a new implementation file.
2.  **Implement Trait**: Implement `BamPlugin` for your struct.
    *   `verify()`: Add auth/TEE check logic.
    *   `build_instructions()`: Construct your program's instructions.
3.  **Mount Plugin**: In `src/main.rs`, replace `ExampleHeartbeatPlugin` with your new instance.
4.  **Update Endpoint**: Ensure the Axum router points to your new payload type.

---

## ⚠️ Critical Constraints

1.  **Atomicity**: Bundles are submitted to Jito as a unit. If one transaction fails, the entire bundle is dropped unless configured otherwise.
2.  **Jito Tips**: The `JitoBundler` automatically handles tip account rotation from the Block Engine. Do not hardcode tip accounts.
3.  **Slot Sensitivity**: Jito bundles are highly sensitive to slot timing. Aggregation intervals should be tuned (`main.rs`) to match block production rates (400ms).

---

## 📡 RPC Dependencies

- **Primary RPC**: Standard Solana RPC for blockhash and account checks.
- **Jito Block Engine**: Specialized endpoint for bundle submission.
- **Photon Indexer**: Required if using `ZkModule` for compressed state.
