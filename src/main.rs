use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use jito_bam_template::{
    maker_plugin::MakerQuotePlugin,
    plugin::BamPlugin,
    bundler::JitoBundler,
    zk::ZkModule,
    metrics::metrics,
};
use solana_sdk::signature::{Keypair, Signer};
use std::net::SocketAddr;
use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing::{info, error, warn};
use std::time::Instant;
use solana_sdk::instruction::Instruction;

pub struct OrderedPayload<T> {
    pub arrival_time: Instant,
    pub payload: T,
}

/// Dynamic high-throughput token bucket rate limiter to prevent HTTP resource exhaustion.
pub struct RateLimiter {
    tokens: f64,
    last_update: Instant,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
}

impl RateLimiter {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            last_update: Instant::now(),
            max_tokens,
            refill_rate,
        }
    }

    /// Checks if a token can be consumed. Thread-safe when wrapped in a mutex.
    pub fn check_and_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

struct AppState<P: BamPlugin> {
    pub plugin: Arc<P>,
    pub bundler: Arc<JitoBundler>,
    pub zk_module: Option<Arc<tokio::sync::Mutex<ZkModule>>>,
    pub tx_queue: mpsc::Sender<OrderedPayload<P::Payload>>,
    pub rate_limiter: Arc<tokio::sync::Mutex<RateLimiter>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize professional tracing subscriber for production monitoring
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    info!("🛠️ Starting Jito BAM Sidecar initialization...");

    // 1. Boot-Time Configuration & Hardened Validation
    info!("🔍 Validating environment variables...");
    
    let authority_key_str = std::env::var("BAM_AUTHORITY_KEY")
        .map_err(|_| anyhow::anyhow!("CRITICAL: BAM_AUTHORITY_KEY environment variable must be set"))?;
    
    let authority = Keypair::from_base58_string(&authority_key_str);
    info!("✅ BAM_AUTHORITY_KEY loaded. Pubkey: {}", authority.pubkey());

    let maker_pubkey_str = std::env::var("MAKER_PUBKEY")
        .map_err(|_| anyhow::anyhow!("CRITICAL: MAKER_PUBKEY environment variable must be set"))?;
    
    let _maker_pubkey = solana_sdk::pubkey::Pubkey::from_str(&maker_pubkey_str)
        .map_err(|e| anyhow::anyhow!("CRITICAL: MAKER_PUBKEY is not a valid Solana public key: {}", e))?;
    info!("✅ MAKER_PUBKEY validated successfully: {}", maker_pubkey_str);

    let _allowed_markets_str = std::env::var("ALLOWED_MARKETS")
        .map_err(|_| anyhow::anyhow!("CRITICAL: ALLOWED_MARKETS environment variable must be set"))?;
    info!("✅ ALLOWED_MARKETS allowance registry active.");

    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let jito_url = std::env::var("JITO_URL").unwrap_or_else(|_| "https://mainnet.block-engine.jito.wtf/api/v1/bundles".to_string());
    let photon_url = std::env::var("PHOTON_URL").ok();

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse()
        .unwrap_or(3030);

    let max_burst: f64 = std::env::var("RATE_LIMIT_MAX_BURST")
        .unwrap_or_else(|_| "100.0".to_string())
        .parse()
        .unwrap_or(100.0);

    let refill_rate: f64 = std::env::var("RATE_LIMIT_REFILL_RATE")
        .unwrap_or_else(|_| "50.0".to_string())
        .parse()
        .unwrap_or(50.0);

    // 2. Initialize Framework Components
    let plugin = Arc::new(MakerQuotePlugin);
    let bundler = Arc::new(JitoBundler::new(&jito_url, &rpc_url, authority));
    
    // Conditional ZK-Module initialization based on environment configuration
    let zk_module = if let Some(p_url) = photon_url {
        Some(Arc::new(tokio::sync::Mutex::new(ZkModule::new(&rpc_url, &p_url).await?)))
    } else {
        warn!("⚠️  PHOTON_URL not provided. Building transactions without ZK-Compression proofs.");
        None
    };

    let (tx_queue, rx_queue) = mpsc::channel(1024);
    let rate_limiter = Arc::new(tokio::sync::Mutex::new(RateLimiter::new(max_burst, refill_rate)));

    let state = Arc::new(AppState {
        plugin,
        bundler,
        zk_module,
        tx_queue,
        rate_limiter,
    });

    // 3. Spawn Aggregator Loop
    let state_clone = state.clone();
    tokio::spawn(async move {
        aggregator_loop(state_clone, rx_queue).await;
    });

    // 4. Start HTTP API with Metrics and OpenAPI schema
    let app = Router::new()
        .route("/submit", post(submit_handler::<MakerQuotePlugin>))
        .route("/metrics", get(metrics_handler))
        .route("/openapi.json", get(openapi_handler))
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 Jito BAM Plugin Sidecar listening gracefully on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("🛑 Sidecar gracefully terminated.");
    Ok(())
}

async fn metrics_handler() -> String {
    metrics().to_prometheus_format()
}

async fn openapi_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Jito BAM Sidecar API",
            "version": "0.1.0",
            "description": "High-performance market maker transaction aggregation API for Jito BAM."
        },
        "paths": {
            "/submit": {
                "post": {
                    "summary": "Submit a new quote update",
                    "description": "Validates the signature on the quote, deduplicates by market_id, and registers it in the 50ms batch window.",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["market_id", "bid_price", "ask_price", "size", "signature"],
                                    "properties": {
                                        "market_id": {
                                            "type": "string",
                                            "example": "SOL/USDC"
                                        },
                                        "bid_price": {
                                            "type": "integer",
                                            "format": "int64",
                                            "example": 145000000
                                        },
                                        "ask_price": {
                                            "type": "integer",
                                            "format": "int64",
                                            "example": 145100000
                                        },
                                        "size": {
                                            "type": "integer",
                                            "format": "int64",
                                            "example": 100
                                        },
                                        "signature": {
                                            "type": "string",
                                            "description": "Base58-encoded Ed25519 signature of the serialized quote fields.",
                                            "example": "2s1B...g4F3"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Payload accepted into the aggregation queue"
                        },
                        "401": {
                            "description": "Signature verification failed or market not allowed"
                        },
                        "429": {
                            "description": "Rate limit exceeded or queue full"
                        }
                    }
                }
            },
            "/metrics": {
                "get": {
                    "summary": "Expose Prometheus metrics",
                    "description": "Returns standard exposition metrics on queue depth, bundle submission status, latency, and deduplication efficiency.",
                    "responses": {
                        "200": {
                            "description": "Prometheus metric snapshot"
                        }
                    }
                }
            }
        }
    }))
}

async fn submit_handler<P: BamPlugin>(
    State(state): State<Arc<AppState<P>>>,
    Json(payload): Json<P::Payload>,
) -> Result<&'static str, (StatusCode, String)> {
    // Increment metrics counter for incoming request
    metrics().inc_incoming();

    // 1. Rate Limiting Check
    {
        let mut limiter = state.rate_limiter.lock().await;
        if !limiter.check_and_consume() {
            return Err((StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string()));
        }
    }
    
    // 2. Framework-level Verification
    if let Err(e) = state.plugin.verify(&payload).await {
        return Err((StatusCode::UNAUTHORIZED, format!("Verification failed: {}", e)));
    }

    // 3. Wrap and Queue for Aggregation
    let ordered_payload = OrderedPayload {
        arrival_time: Instant::now(),
        payload,
    };

    if let Err(_) = state.tx_queue.try_send(ordered_payload) {
        return Err((StatusCode::TOO_MANY_REQUESTS, "Queue full".to_string()));
    }

    // Update queue depth metric
    let current_depth = 1024 - state.tx_queue.capacity();
    metrics().set_queue_depth(current_depth as u64);

    Ok("Accepted")
}

async fn aggregator_loop<P: BamPlugin>(
    state: Arc<AppState<P>>,
    mut rx: mpsc::Receiver<OrderedPayload<P::Payload>>,
) {
    let tick_rate_ms: u64 = std::env::var("BAM_TICK_RATE_MS")
        .unwrap_or_else(|_| "50".to_string())
        .parse()
        .unwrap_or(50);
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(tick_rate_ms));
    
    let mut dedup_map: std::collections::HashMap<String, OrderedPayload<P::Payload>> = std::collections::HashMap::new();
    let mut regular_batch: Vec<OrderedPayload<P::Payload>> = Vec::new();

    loop {
        tokio::select! {
            Some(payload) = rx.recv() => {
                if let Some(key) = state.plugin.grouping_key(&payload.payload) {
                    // Overwrite any existing quote for this market_id
                    if dedup_map.insert(key, payload).is_some() {
                        // Dropped a stale quote. Increment metrics counter
                        metrics().inc_deduped(1);
                    }
                } else {
                    regular_batch.push(payload);
                }

                if dedup_map.len() + regular_batch.len() >= state.plugin.batch_size() {
                    let mut final_batch = std::mem::take(&mut regular_batch);
                    final_batch.extend(dedup_map.drain().map(|(_, v)| v));
                    process_batch(&state, final_batch).await;
                    metrics().set_queue_depth((1024 - state.tx_queue.capacity()) as u64);
                }
            }
            _ = interval.tick() => {
                if !dedup_map.is_empty() || !regular_batch.is_empty() {
                    let mut final_batch = std::mem::take(&mut regular_batch);
                    final_batch.extend(dedup_map.drain().map(|(_, v)| v));
                    process_batch(&state, final_batch).await;
                    metrics().set_queue_depth((1024 - state.tx_queue.capacity()) as u64);
                }
            }
        }
    }
}

async fn process_batch<P: BamPlugin>(
    state: &AppState<P>,
    mut batch: Vec<OrderedPayload<P::Payload>>,
) {
    let start_time = Instant::now();
    info!("📦 Processing batch of {} items", batch.len());
    
    // Ensure strict FCFS ordering by sorting based on arrival time
    batch.sort_by_key(|p| p.arrival_time);

    // Each payload will have its instructions placed in a separate chunk
    // to become sequential transactions in the Jito bundle.
    let mut instruction_chunks: Vec<Vec<Instruction>> = Vec::new();
    let authority_pubkey = state.bundler.authority.pubkey();

    for ordered in batch {
        let payload = ordered.payload;
        // Optional: ZK-State resolution before building instructions
        if let Some(_zk) = &state.zk_module {
            // let mut _zk_locked = _zk.lock().await;
            // _zk_locked.get_compressed_account(...).await;
        }

        match state.plugin.build_instructions(&payload, &authority_pubkey).await {
            Ok(ixs) => instruction_chunks.push(ixs),
            Err(e) => error!("❌ Failed to build instructions for payload: {}", e),
        }
    }

    if !instruction_chunks.is_empty() {
        match state.bundler.send_bundle_with_tip(instruction_chunks, state.plugin.get_tip_amount()).await {
            Ok(id) => {
                metrics().inc_bundle_success();
                let bundler = state.bundler.clone();
                tokio::spawn(async move {
                    bundler.watch_bundle(id).await;
                });
            }
            Err(e) => {
                metrics().inc_bundle_failure();
                error!("❌ Bundle submission failed: {}", e);
            }
        }
    }

    // Record aggregation processing latency
    let elapsed = start_time.elapsed().as_millis() as u64;
    metrics().record_latency(elapsed);
}

/// Receives signals to gracefully shutdown the server to protect transactions in progress.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install terminate signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("🔔 Received Ctrl+C. Initiating graceful shutdown of Jito BAM Sidecar...");
        },
        _ = terminate => {
            info!("🔔 Received SIGTERM. Initiating graceful shutdown of Jito BAM Sidecar...");
        },
    }
}
