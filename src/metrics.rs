use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// High-performance system metrics using atomic values for lock-free, zero-overhead updates.
pub struct SystemMetrics {
    pub incoming_requests: AtomicU64,
    pub deduped_updates: AtomicU64,
    pub bundle_submissions_success: AtomicU64,
    pub bundle_submissions_failure: AtomicU64,
    pub queue_depth: AtomicU64,
    pub processing_latency_sum_ms: AtomicU64,
    pub processing_latency_count: AtomicU64,
}

impl SystemMetrics {
    pub fn new() -> Self {
        Self {
            incoming_requests: AtomicU64::new(0),
            deduped_updates: AtomicU64::new(0),
            bundle_submissions_success: AtomicU64::new(0),
            bundle_submissions_failure: AtomicU64::new(0),
            queue_depth: AtomicU64::new(0),
            processing_latency_sum_ms: AtomicU64::new(0),
            processing_latency_count: AtomicU64::new(0),
        }
    }

    /// Increments incoming request count
    pub fn inc_incoming(&self) {
        self.incoming_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments deduped/dropped update count
    pub fn inc_deduped(&self, amount: u64) {
        self.deduped_updates.fetch_add(amount, Ordering::Relaxed);
    }

    /// Increments successful bundle submissions
    pub fn inc_bundle_success(&self) {
        self.bundle_submissions_success
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increments failed bundle submissions
    pub fn inc_bundle_failure(&self) {
        self.bundle_submissions_failure
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Sets the queue depth gauge
    pub fn set_queue_depth(&self, val: u64) {
        self.queue_depth.store(val, Ordering::Relaxed);
    }

    /// Records latency in milliseconds
    pub fn record_latency(&self, ms: u64) {
        self.processing_latency_sum_ms
            .fetch_add(ms, Ordering::Relaxed);
        self.processing_latency_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Serializes active metrics into standard Prometheus exposition format.
    pub fn to_prometheus_format(&self) -> String {
        let incoming = self.incoming_requests.load(Ordering::Relaxed);
        let deduped = self.deduped_updates.load(Ordering::Relaxed);
        let success = self.bundle_submissions_success.load(Ordering::Relaxed);
        let failure = self.bundle_submissions_failure.load(Ordering::Relaxed);
        let q_depth = self.queue_depth.load(Ordering::Relaxed);
        let latency_sum = self.processing_latency_sum_ms.load(Ordering::Relaxed);
        let latency_count = self.processing_latency_count.load(Ordering::Relaxed);

        let avg_latency = if latency_count > 0 {
            (latency_sum as f64) / (latency_count as f64)
        } else {
            0.0
        };

        format!(
            "# HELP jito_bam_incoming_requests_total Total number of incoming quote requests.\n\
             # TYPE jito_bam_incoming_requests_total counter\n\
             jito_bam_incoming_requests_total {}\n\n\
             # HELP jito_bam_deduped_updates_total Total number of dropped (deduplicated) stale quotes.\n\
             # TYPE jito_bam_deduped_updates_total counter\n\
             jito_bam_deduped_updates_total {}\n\n\
             # HELP jito_bam_bundle_submissions_total Total Jito bundle submissions tracked by status.\n\
             # TYPE jito_bam_bundle_submissions_total counter\n\
             jito_bam_bundle_submissions_total{{status=\"success\"}} {}\n\
             jito_bam_bundle_submissions_total{{status=\"failure\"}} {}\n\n\
             # HELP jito_bam_queue_depth Current size of the aggregation queue.\n\
             # TYPE jito_bam_queue_depth gauge\n\
             jito_bam_queue_depth {}\n\n\
             # HELP jito_bam_processing_latency_avg_ms Average aggregation and processing latency in milliseconds.\n\
             # TYPE jito_bam_processing_latency_avg_ms gauge\n\
             jito_bam_processing_latency_avg_ms {:.2}\n",
            incoming, deduped, success, failure, q_depth, avg_latency
        )
    }
}

pub static METRICS: OnceLock<SystemMetrics> = OnceLock::new();

/// Returns the global reference to high-throughput system metrics.
pub fn metrics() -> &'static SystemMetrics {
    METRICS.get_or_init(SystemMetrics::new)
}
