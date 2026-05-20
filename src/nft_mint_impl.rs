use crate::plugin::BamPlugin;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// A generic payload for an NFT Mint or Limited Sale
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NftMintPayload {
    pub buyer_wallet: String,
    pub collection_id: String,
    pub quantity: u8,
}

/// A concrete example of using the BAM Plugin for an NFT Mint.
pub struct ExampleNftMintPlugin;

#[async_trait]
impl BamPlugin for ExampleNftMintPlugin {
    // 1. Define your custom payload type
    type Payload = NftMintPayload;

    fn id(&self) -> &'static str {
        "nft-mint-fcfs-plugin"
    }

    async fn verify(&self, _payload: &Self::Payload) -> Result<()> {
        // Here you would verify things like:
        // - Did the buyer sign this payload?
        // - Is the collection ID valid?
        // - Are we sold out?
        Ok(())
    }

    async fn build_instructions(
        &self,
        payload: &Self::Payload,
        authority: &Pubkey,
    ) -> Result<Vec<Instruction>> {
        // 2. Build the exact Solana instructions for your specific program
        // This could be Metaplex, a custom game program, or a DEX.

        // Example: Dummy NFT Program ID
        let program_id = Pubkey::new_from_array([1u8; 32]);

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(*authority, true), // The sidecar paying for the mint
                AccountMeta::new(payload.buyer_wallet.parse()?, false), // The buyer receiving the NFT
            ],
            data: bincode::serialize(&payload.quantity)?, // Tell the program how many to mint
        };

        Ok(vec![ix])
    }
}
