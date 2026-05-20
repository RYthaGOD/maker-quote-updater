use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use anyhow::Result;

/// Core trait for defining Jito BAM Plugin logic.
/// Implement this trait to customize payload verification and transaction construction.
#[async_trait]
pub trait BamPlugin: Send + Sync + 'static {
    /// The payload type received from the frontend (Axum/gRPC).
    type Payload: Serialize + DeserializeOwned + Send + Sync + Clone;

    /// Unique identifier for the plugin.
    fn id(&self) -> &'static str;

    /// Verify the incoming payload (e.g., TEE signature, rate limits).
    async fn verify(&self, payload: &Self::Payload) -> Result<()>;

    /// Return a key to group payloads by (used for deduplication).
    fn grouping_key(&self, _payload: &Self::Payload) -> Option<String> {
        None
    }

    /// Build the Solana instructions for a specific payload.
    /// This is where the "Business Logic" lives.
    async fn build_instructions(
        &self, 
        payload: &Self::Payload,
        authority: &Pubkey,
    ) -> Result<Vec<Instruction>>;

    /// Optional: Custom tip amount in lamports for this specific payload.
    fn get_tip_amount(&self) -> u64 {
        5_000 // Default Jito tip
    }

    /// Optional: Batching criteria. How many payloads per bundle?
    fn batch_size(&self) -> usize {
        10
    }
}
