# 🤖 Agent Action Recipes

Follow these strictly formatted instructions to perform common tasks in this repository.

---

## Recipe: Adding a New Plugin Strategy

**Objective**: Create a new BAM plugin for a specific application logic.

1.  **File Creation**: Create `src/strategies/<name>.rs`.
2.  **Define Payload**:
    ```rust
    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct MyPayload { ... }
    ```
3.  **Implement Trait**:
    ```rust
    #[async_trait]
    impl BamPlugin for MyStrategy {
        type Payload = MyPayload;
        fn id(&self) -> &'static str { "..." }
        async fn verify(&self, payload: &Self::Payload) -> Result<()> { ... }
        async fn build_instructions(...) -> Result<Vec<Instruction>> { ... }
    }
    ```
4.  **Integration**:
    *   In `main.rs`, import your new strategy.
    *   Update `AppState` to use your strategy.
    *   Update `submit_handler` generics.

---

## Recipe: Hardening TEE Verification

**Objective**: Implement cryptographic signature checks for off-chain payloads.

1.  **Dependency**: Use `ed25519-dalek` (already in `Cargo.toml`).
2.  **Logic**:
    *   Add `signature: String` to your payload struct.
    *   In `BamPlugin::verify()`, decode the signature and public key.
    *   Reconstruct the message buffer exactly as it was signed.
    *   Perform `verifying_key.verify(message, &signature)`.

---

## Recipe: Tuning Aggregation

**Objective**: Adjust performance for different load profiles.

1.  **Batch Size**: Override `fn batch_size(&self) -> usize` in your plugin implementation (Default: 10).
2.  **Flush Interval**: Modify `tokio::time::interval()` in `main.rs` (Default: 5 seconds).
3.  **Queue Size**: Modify `mpsc::channel(1024)` in `main.rs` to handle higher/lower burst traffic.

---

## Recipe: Deploying to a New Jito Region

**Objective**: Minimize latency by targeting specific Block Engine endpoints.

1.  **Update `.env`**: Change `JITO_URL` to one of:
    *   `https://frankfurt.mainnet.block-engine.jito.wtf/api/v1/bundles` (EU)
    *   `https://ny.mainnet.block-engine.jito.wtf/api/v1/bundles` (US)
    *   `https://tokyo.mainnet.block-engine.jito.wtf/api/v1/bundles` (Asia)
2.  **Monitor**: Use `cargo run` and watch for "Bundle Accepted" logs.
