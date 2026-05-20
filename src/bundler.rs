#![allow(deprecated)]
use jito_sdk_rust::JitoJsonRpcSDK;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use tracing::{info, warn};
use anyhow::{Result, anyhow};
use std::sync::Arc;

/// High-performance interface for Jito Bundle management.
/// Responsible for transaction construction, tip account resolution, and Block Engine integration.
pub struct JitoBundler {
    pub jito_sdk: JitoJsonRpcSDK,
    pub rpc_client: Arc<RpcClient>,
    pub authority: Keypair,
}

impl JitoBundler {
    pub fn new(jito_url: &str, rpc_url: &str, authority: Keypair) -> Self {
        Self {
            jito_sdk: JitoJsonRpcSDK::new(jito_url, None),
            rpc_client: Arc::new(RpcClient::new(rpc_url)),
            authority,
        }
    }

    #[allow(deprecated)]
    pub async fn send_bundle_with_tip(
        &self,
        instruction_chunks: Vec<Vec<Instruction>>,
        tip_lamports: u64,
    ) -> Result<String> {
        let blockhash = self.rpc_client.get_latest_blockhash()?;
        
        // 1. Fetch Jito Tip Account (with static fallback to avoid single RPC point of failure)
        let tip_account_str = match self.jito_sdk.get_random_tip_account().await {
            Ok(account) => account,
            Err(e) => {
                warn!("⚠️ Failed to fetch random Jito tip account over RPC: {}. Using static mainnet fallback.", e);
                let static_tips = [
                    "96gYZz2EBfmgvRLE31Atmrx2dB1i816auBiLocADrj2y",
                    "HFqU5x63VT43qWN6586g5FcP5C9149W4yFiChmGg7v8y",
                    "Cw8CFBTj43VWdDXssC7mWwbAHwBiyeWdG1XXc5VNV24q",
                    "ADaUMo7t4q4W36d7Qv3fgWok2H6Cq7XTX7mSQc3gWwJ1",
                    "ADuUk9ZGLFrK224UBZ1PsecS35SLtPWd14V42FstE3b",
                    "DttWaSB8KM1HDy47tQ65G3hGjvWh5rqkgGSmJ5iXm8bB",
                    "3AVaG8o6kBJjH2mmci2GY56dnpa2221zR485DeXSmSw1",
                ];
                let idx = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as usize % static_tips.len();
                static_tips[idx].to_string()
            }
        };
        
        let tip_pubkey: Pubkey = tip_account_str.parse()
            .map_err(|_| anyhow!("Invalid tip pubkey: {}", tip_account_str))?;

        // 2. Prepare Transactions
        let mut txs = Vec::new();

        // Payload transactions (FCFS sequential chunks)
        for chunk in instruction_chunks {
            let chunk_tx = Transaction::new_signed_with_payer(
                &chunk,
                Some(&self.authority.pubkey()),
                &[&self.authority],
                blockhash,
            );
            txs.push(chunk_tx);
        }

        if txs.len() > 4 {
            warn!("⚠️ Bundle contains {} payload transactions. Jito limits bundles to 5 total transactions (4 payloads + 1 tip). Submission may fail if it exceeds limits.", txs.len());
        }

        // Tip transaction
        let tip_tx = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &self.authority.pubkey(),
                &tip_pubkey,
                tip_lamports,
            )],
            Some(&self.authority.pubkey()),
            &[&self.authority],
            blockhash,
        );
        txs.push(tip_tx);

        // 3. Serialize and Send
        let bundle: Vec<String> = txs
            .iter()
            .map(|tx| bs58::encode(bincode::serialize(tx).unwrap()).into_string())
            .collect();

        let params = serde_json::json!(bundle);

        let response = self.jito_sdk.send_bundle(Some(params), None).await
            .map_err(|e| anyhow!("Bundle rejected: {}", e))?;
            
        let bundle_id = response["result"].as_str().unwrap_or("unknown_id").to_string();

        info!("🚀 Bundle {} submitted to Jito.", bundle_id);
        Ok(bundle_id)
    }

    pub async fn watch_bundle(&self, bundle_id: String) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        for _ in 0..15 {
            interval.tick().await;
            if let Ok(response) = self.jito_sdk.get_bundle_statuses(vec![bundle_id.clone()]).await {
                if let Some(statuses) = response["result"]["value"].as_array() {
                    if let Some(s) = statuses.first() {
                        if let Some(status) = s["confirmation_status"].as_str() {
                            match status {
                                "confirmed" | "finalized" | "processed" => {
                                    info!("🎉 Bundle {} CONFIRMED!", bundle_id);
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        warn!("⌛ Bundle {} status unknown after timeout.", bundle_id);
    }
}
