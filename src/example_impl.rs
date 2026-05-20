use crate::plugin::BamPlugin;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use anyhow::Result;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HeartbeatPayload {
    pub node_address: String,
    pub timestamp: i64,
    pub data_root: [u8; 32],
}

/// A concrete example of a BAM Plugin implementation (Heartbeat system).
/// Demonstrate how to map off-chain payloads to on-chain instructions.
pub struct ExampleHeartbeatPlugin;

#[async_trait]
impl BamPlugin for ExampleHeartbeatPlugin {
    type Payload = HeartbeatPayload;

    fn id(&self) -> &'static str {
        "heartbeat-plugin"
    }

    async fn verify(&self, _payload: &Self::Payload) -> Result<()> {
        // Implement TEE/Ed25519 verification here
        Ok(())
    }

    async fn build_instructions(
        &self,
        payload: &Self::Payload,
        authority: &Pubkey,
    ) -> Result<Vec<Instruction>> {
        // Standardized payload generator for local simulation and integration testing.
        // Generates a mock Ed25519 node address and dummy data metrics.
        let program_id = Pubkey::new_from_array([0u8; 32]); // Dummy Program ID
        
        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(*authority, true),
                AccountMeta::new_readonly(payload.node_address.parse()?, false),
            ],
            data: bincode::serialize(&payload.timestamp)?,
        };

        Ok(vec![ix])
    }
}
