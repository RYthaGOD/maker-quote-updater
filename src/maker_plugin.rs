use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use anyhow::{anyhow, Result};
use crate::plugin::BamPlugin;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuoteUpdate {
    pub market_id: String,
    pub bid_price: u64,
    pub ask_price: u64,
    pub size: u64,
    pub timestamp_ms: u64, // Unix timestamp in milliseconds for replay protection
    pub signature: String, // Base58 encoded signature
}

#[derive(Serialize)]
struct QuoteUpdateData<'a> {
    market_id: &'a str,
    bid_price: u64,
    ask_price: u64,
    size: u64,
    timestamp_ms: u64,
}

pub struct MakerQuotePlugin;

#[async_trait]
impl BamPlugin for MakerQuotePlugin {
    type Payload = QuoteUpdate;

    fn id(&self) -> &'static str {
        "maker_quote_updater"
    }

    async fn verify(&self, payload: &Self::Payload) -> Result<()> {
        // 1. Verify against allow-list
        let allowed_markets_str = std::env::var("ALLOWED_MARKETS").unwrap_or_else(|_| "".to_string());
        let allowed_markets: Vec<&str> = allowed_markets_str.split(',').collect();
        
        if !allowed_markets.contains(&payload.market_id.as_str()) {
            return Err(anyhow!("Market {} is not in the ALLOWED_MARKETS list", payload.market_id));
        }

        // 2. Load Pubkey and Signature
        let pubkey_base58 = std::env::var("MAKER_PUBKEY").map_err(|_| anyhow!("MAKER_PUBKEY environment variable not set"))?;
        let pubkey = Pubkey::from_str(&pubkey_base58).map_err(|e| anyhow!("Invalid MAKER_PUBKEY: {}", e))?;

        let signature = Signature::from_str(&payload.signature).map_err(|e| anyhow!("Invalid bs58 signature: {}", e))?;

        // 3. Verify Timestamp (Replay Protection)
        let current_time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Strict 5-second replay window (allows for minor network lag or server/client clock drift)
        let diff = if current_time_ms >= payload.timestamp_ms {
            current_time_ms - payload.timestamp_ms
        } else {
            payload.timestamp_ms - current_time_ms
        };

        if diff > 5000 {
            return Err(anyhow!(
                "Replay protection triggered: payload timestamp is stale. current_time: {}, payload_time: {} (diff: {}ms, max allowed: 5000ms)",
                current_time_ms,
                payload.timestamp_ms,
                diff
            ));
        }

        // 4. Serialize Data Payload
        let data = QuoteUpdateData {
            market_id: &payload.market_id,
            bid_price: payload.bid_price,
            ask_price: payload.ask_price,
            size: payload.size,
            timestamp_ms: payload.timestamp_ms,
        };
        let serialized_data = bincode::serialize(&data).unwrap();

        // 5. Verify Signature
        if !signature.verify(pubkey.as_ref(), &serialized_data) {
            return Err(anyhow!("Signature verification failed"));
        }

        Ok(())
    }

    fn grouping_key(&self, payload: &Self::Payload) -> Option<String> {
        Some(payload.market_id.clone())
    }

    async fn build_instructions(
        &self,
        payload: &Self::Payload,
        authority: &Pubkey,
    ) -> Result<Vec<Instruction>> {
        // Example: Build an instruction to update the quote on a hypothetical DEX
        // This program ID would be the DEX's actual program ID.
        let program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        
        let market_pubkey = Pubkey::from_str(&payload.market_id).unwrap_or(*authority);

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(market_pubkey, false),
                AccountMeta::new(*authority, true),
            ],
            data: bincode::serialize(payload).unwrap_or_default(),
        };

        Ok(vec![ix])
    }

    fn batch_size(&self) -> usize {
        20 // E.g., batch up to 20 quotes per bundle
    }
}
