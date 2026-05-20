import nacl from 'tweetnacl';
import bs58 from 'bs58';

interface QuoteUpdate {
    market_id: string;
    bid_price: bigint;
    ask_price: bigint;
    size: bigint;
    timestamp_ms: bigint;
}

/**
 * Serializes the QuoteUpdate payload into the exact binary format expected by Rust's bincode.
 * 
 * Layout:
 * 1. String length (u64, little-endian) - 8 bytes
 * 2. String bytes (UTF-8) - length bytes
 * 3. bid_price (u64, little-endian) - 8 bytes
 * 4. ask_price (u64, little-endian) - 8 bytes
 * 5. size (u64, little-endian) - 8 bytes
 * 6. timestamp_ms (u64, little-endian) - 8 bytes
 */
function serializeBincode(quote: QuoteUpdate): Uint8Array {
    const stringBytes = Buffer.from(quote.market_id, 'utf-8');
    const bufferSize = 8 + stringBytes.length + 8 + 8 + 8 + 8;
    const buffer = Buffer.alloc(bufferSize);
    
    let offset = 0;

    // 1. String length prefix (u64 LE)
    buffer.writeBigUInt64LE(BigInt(stringBytes.length), offset);
    offset += 8;

    // 2. String bytes
    stringBytes.copy(buffer, offset);
    offset += stringBytes.length;

    // 3. bid_price (u64 LE)
    buffer.writeBigUInt64LE(quote.bid_price, offset);
    offset += 8;

    // 4. ask_price (u64 LE)
    buffer.writeBigUInt64LE(quote.ask_price, offset);
    offset += 8;

    // 5. size (u64 LE)
    buffer.writeBigUInt64LE(quote.size, offset);
    offset += 8;

    // 6. timestamp_ms (u64 LE)
    buffer.writeBigUInt64LE(quote.timestamp_ms, offset);

    return new Uint8Array(buffer);
}

async function run() {
    // 1. Load the private key of the Maker Bot (using seed [1u8; 32] as in generate_payload)
    const seed = new Uint8Array(32).fill(1);
    const keypair = nacl.sign.keypair.fromSeed(seed);
    const publicKeyBase58 = bs58.encode(keypair.publicKey);
    
    console.log('Maker Public Key (Base58):', publicKeyBase58);

    // 2. Construct the quote update payload
    const quote: QuoteUpdate = {
        market_id: 'SOL/USDC',
        bid_price: 145000000n, // $145.000000
        ask_price: 145100000n, // $145.100000
        size: 100n,
        timestamp_ms: BigInt(Date.now()), // unix epoch ms
    };

    // 3. Serialize to bincode format
    const serialized = serializeBincode(quote);

    // 4. Cryptographically sign the serialized data
    const signatureBytes = nacl.sign.detached(serialized, keypair.secretKey);
    const signatureBase58 = bs58.encode(signatureBytes);

    console.log('Generated Signature (Base58):', signatureBase58);

    // 5. Build full JSON payload for HTTP submit
    const httpPayload = {
        market_id: quote.market_id,
        bid_price: Number(quote.bid_price),
        ask_price: Number(quote.ask_price),
        size: Number(quote.size),
        timestamp_ms: Number(quote.timestamp_ms),
        signature: signatureBase58,
    };

    console.log('\n--- Payload JSON to send to Jito BAM Sidecar ---');
    console.log(JSON.stringify(httpPayload, null, 2));

    console.log('\n--- Curl Command ---');
    console.log(`curl -X POST http://localhost:3030/submit \\
  -H 'Content-Type: application/json' \\
  -d '${JSON.stringify(httpPayload)}'`);
}

run().catch(console.error);
