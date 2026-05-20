use bs58;
use ed25519_dalek::Signer;
use serde::Serialize;

#[derive(Serialize)]
struct QuoteUpdateData<'a> {
    market_id: &'a str,
    bid_price: u64,
    ask_price: u64,
    size: u64,
    timestamp_ms: u64,
}

#[derive(Serialize)]
struct FullPayload<'a> {
    market_id: &'a str,
    bid_price: u64,
    ask_price: u64,
    size: u64,
    timestamp_ms: u64,
    signature: String,
}

fn main() {
    // 1. Generate a dummy Keypair for the MM bot (Fixed bytes for testing)
    let bytes = [1u8; 32];
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&bytes);
    let pubkey = signing_key.verifying_key();

    let pubkey_base58 = bs58::encode(pubkey.as_bytes()).into_string();
    println!("--- MAKER BOT KEY ---");
    println!("Set this in your .env: MAKER_PUBKEY={}\n", pubkey_base58);

    // 2. The Data to sign
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

    // 3. Serialize and Sign
    let serialized_data = bincode::serialize(&data).unwrap();
    let signature = signing_key.sign(&serialized_data);
    let signature_base58 = bs58::encode(signature.to_bytes()).into_string();

    // 4. Create Full Payload
    let payload = FullPayload {
        market_id,
        bid_price,
        ask_price,
        size,
        timestamp_ms,
        signature: signature_base58,
    };

    println!("--- CURL TEST COMMAND ---");
    let json = serde_json::to_string_pretty(&payload).unwrap();
    println!("curl -X POST http://localhost:3030/submit \\");
    println!("  -H 'Content-Type: application/json' \\");
    println!("  -d '{}'", json);
}
