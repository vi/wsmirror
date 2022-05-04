#[derive(prometheus_metric_storage::MetricStorage)]
#[metric(subsystem = "wsmirror")]
pub struct Metrics {
    /// Number of clients connection events
    pub connections: prometheus::IntCounter,

    /// Number of clients that reached WebSocket phase of the connection
    pub ws_connections: prometheus::IntCounter,

    /// Number of clients disconnection events
    pub disconnections: prometheus::IntCounter,

    /// Number of client connections immeidately rejected because of overload
    pub rejected_overload: prometheus::IntCounter,

    /// Number of client connections immeidately rejected without even trying to reply anything to them
    pub rejected_quick: prometheus::IntCounter,

    /// Number text WebSocket messages from clients
    pub text_messages: prometheus::IntCounter,

    /// Number binary WebSocket messages from clients
    pub binary_messages: prometheus::IntCounter,

    /// Number ping WebSocket messages from clients
    pub ping_messages: prometheus::IntCounter,

    /// Number pong WebSocket messages from clients
    pub pong_messages: prometheus::IntCounter,

    /// Number close WebSocket messages from clients
    pub close_messages: prometheus::IntCounter,

    /// Number close codes from WebSocket messages from clients
    pub close_codes: prometheus::IntCounterVec,

    /// Number other WebSocket messages from clients
    pub other_messages: prometheus::IntCounter,

    /// Number pings we have sent to clients
    pub pings_sent: prometheus::IntCounter,

    /// Number closed client connections due to ping timeout
    pub ping_timeouts: prometheus::IntCounter,

    /// Number of total bytes in client's text, binary and ping messages
    pub bytes_from_clients: prometheus::IntCounter,

    /// Number of finished client sessions by session duration in seconds
    #[metric(buckets(0.2, 1.1, 11, 61, 3601, 86401, 604800))]
    pub session_duration_seconds: prometheus::Histogram,

    /// Number of finished client sessions by total received messages
    #[metric(buckets(1, 2, 11, 0x40, 0x800, 0x10000, 0x200000, 0x4000000))]
    pub session_messages_counts: prometheus::Histogram,

    /// Number of finished client sessions by total received messages
    #[metric(buckets(0x20, 0x400, 0x8000, 0x100000, 0x2000000, 0x40000000))]
    pub session_bytes: prometheus::Histogram,
}

pub fn new() -> Metrics {
    Metrics::new(prometheus::default_registry()).unwrap()
}

pub struct ClientSessionMetrics {
    pub bytes: u64,
    pub msgs: u64,
    _duration: prometheus::HistogramTimer,
    metrics: &'static Metrics,
}

impl ClientSessionMetrics {
    pub fn new(metrics: &'static Metrics) -> ClientSessionMetrics {
        ClientSessionMetrics {
            bytes: 0,
            msgs: 0,
            _duration: metrics.session_duration_seconds.start_timer(),
            metrics,
        }
    }
}

impl Drop for ClientSessionMetrics {
    fn drop(&mut self) {
        self.metrics.session_messages_counts.observe(self.msgs as f64);
        self.metrics.session_bytes.observe(self.bytes as f64);
    }
}
