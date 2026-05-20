use ed25519_dalek::Signer;
use jito_bam_template::{
    maker_plugin::{MakerQuotePlugin, QuoteUpdate},
    metrics::metrics,
    plugin::BamPlugin,
};

#[derive(serde::Serialize)]
struct QuoteUpdateData<'a> {
    market_id: &'a str,
    bid_price: u64,
    ask_price: u64,
    size: u64,
    timestamp_ms: u64,
}

#[tokio::test]
async fn test_maker_quote_verification() {
    // 1. Generate test keypair
    let bytes = [42u8; 32]; // Seed
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&bytes);
    let verifying_key = signing_key.verifying_key();
    let pubkey_base58 = bs58::encode(verifying_key.as_bytes()).into_string();

    // Set env variables for tests
    std::env::set_var("MAKER_PUBKEY", &pubkey_base58);
    std::env::set_var("ALLOWED_MARKETS", "SOL/USDC,BTC/USDC");

    let plugin = MakerQuotePlugin;

    // 2. Build signed payload
    let market_id = "SOL/USDC";
    let bid_price = 145_000_000;
    let ask_price = 145_100_000;
    let size = 100;
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let data = QuoteUpdateData {
        market_id,
        bid_price,
        ask_price,
        size,
        timestamp_ms,
    };
    let serialized_data = bincode::serialize(&data).unwrap();
    let signature = signing_key.sign(&serialized_data);
    let signature_base58 = bs58::encode(signature.to_bytes()).into_string();

    let valid_payload = QuoteUpdate {
        market_id: market_id.to_string(),
        bid_price,
        ask_price,
        size,
        timestamp_ms,
        signature: signature_base58,
    };

    // 3. Verify valid signature passes
    let res = plugin.verify(&valid_payload).await;
    assert!(res.is_ok(), "Valid signed quote must pass verification");

    // 4. Verify invalid signature fails
    let mut invalid_payload = valid_payload.clone();
    invalid_payload.signature = "2s1BinvalidSignatureHereXXXXXXXXXXXX".to_string();
    let res_invalid = plugin.verify(&invalid_payload).await;
    assert!(
        res_invalid.is_err(),
        "Invalid signature must fail verification"
    );

    // 5. Verify unallowed market is rejected
    let mut unallowed_payload = valid_payload.clone();
    unallowed_payload.market_id = "ETH/USDC".to_string();
    // Need to sign this new market payload to isolate allow-list validation
    let data_unallowed = QuoteUpdateData {
        market_id: "ETH/USDC",
        bid_price,
        ask_price,
        size,
        timestamp_ms,
    };
    let serialized_unallowed = bincode::serialize(&data_unallowed).unwrap();
    let sig_unallowed = signing_key.sign(&serialized_unallowed);
    unallowed_payload.signature = bs58::encode(sig_unallowed.to_bytes()).into_string();

    let res_unallowed = plugin.verify(&unallowed_payload).await;
    assert!(
        res_unallowed.is_err(),
        "Unallowed market must be rejected by allow-list"
    );

    // 6. Verify stale/replay timestamp is rejected (10 seconds ago)
    let mut stale_payload = valid_payload.clone();
    let stale_timestamp_ms = timestamp_ms - 10000;
    stale_payload.timestamp_ms = stale_timestamp_ms;

    let data_stale = QuoteUpdateData {
        market_id,
        bid_price,
        ask_price,
        size,
        timestamp_ms: stale_timestamp_ms,
    };
    let serialized_stale = bincode::serialize(&data_stale).unwrap();
    let sig_stale = signing_key.sign(&serialized_stale);
    stale_payload.signature = bs58::encode(sig_stale.to_bytes()).into_string();

    let res_stale = plugin.verify(&stale_payload).await;
    assert!(
        res_stale.is_err(),
        "Stale/replayed timestamp must be rejected"
    );
}

#[test]
fn test_maker_quote_grouping_key() {
    let plugin = MakerQuotePlugin;
    let payload = QuoteUpdate {
        market_id: "SOL/USDC".to_string(),
        bid_price: 100,
        ask_price: 101,
        size: 10,
        timestamp_ms: 0,
        signature: "".to_string(),
    };

    let key = plugin.grouping_key(&payload);
    assert_eq!(
        key,
        Some("SOL/USDC".to_string()),
        "Grouping key must be market_id for deduplication"
    );
}

#[test]
fn test_system_metrics_exposition() {
    let m = metrics();
    m.inc_incoming();
    m.inc_deduped(5);
    m.inc_bundle_success();
    m.inc_bundle_failure();
    m.set_queue_depth(12);
    m.record_latency(45);

    let format = m.to_prometheus_format();
    assert!(
        format.contains("jito_bam_incoming_requests_total"),
        "Incoming requests metric invalid"
    );
    assert!(
        format.contains("jito_bam_deduped_updates_total"),
        "Deduped updates metric invalid"
    );
    assert!(
        format.contains("jito_bam_bundle_submissions_total{status=\"success\"}"),
        "Bundle success metric invalid"
    );
    assert!(
        format.contains("jito_bam_bundle_submissions_total{status=\"failure\"}"),
        "Bundle failure metric invalid"
    );
    assert!(
        format.contains("jito_bam_queue_depth 12"),
        "Queue depth metric invalid"
    );
    assert!(
        format.contains("jito_bam_processing_latency_avg_ms"),
        "Processing latency average metric invalid"
    );
}
