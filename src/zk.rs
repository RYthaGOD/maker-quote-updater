use anyhow::Result;
use light_client::indexer::Indexer;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use solana_sdk::pubkey::Pubkey;
use tracing::{error, info};

/// Modular interface for ZK-Compression (Light Protocol) operations.
/// Handles compressed state resolution and proof verification logic.
pub struct ZkModule {
    pub client: LightClient,
}

impl ZkModule {
    /// Initialize the ZK module with RPC and Photon (Indexer) URLs.
    pub async fn new(rpc_url: &str, photon_url: &str) -> Result<Self> {
        let config = LightClientConfig::new(rpc_url.to_string(), Some(photon_url.to_string()));
        let client = LightClient::new(config).await?;
        info!("💠 ZK-Compression Module initialized.");
        Ok(Self { client })
    }

    /// Resolve a compressed account and verify its existence.
    pub async fn get_compressed_account(&mut self, address: Pubkey) -> Result<()> {
        match self
            .client
            .get_compressed_account(address.to_bytes(), None)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("❌ Failed to resolve compressed account {}: {}", address, e);
                Err(e.into())
            }
        }
    }

    // Additional helper methods for ZK-proof generation can be added here.
}
