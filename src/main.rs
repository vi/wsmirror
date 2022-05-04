use std::net::SocketAddr;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;

#[cfg(feature = "metrics")]
mod metrics;

static CLIENT_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
const MAX_CLIENTS: u32 = 2000;

const PING_SECONDS: u64 = 5;
const PING_TIMEOUT: u64 = 12;

async fn serve_client(
    socket: tokio::net::TcpStream,
    #[cfg(feature = "metrics")] metrics: &'static metrics::Metrics,
) -> anyhow::Result<()> {
    macro_rules! m {
        ($e:expr) => {
            #[cfg(feature = "metrics")]
            $e
        };
    }
    #[cfg(feature = "metrics")]
    let mut client_session_metrics = metrics::ClientSessionMetrics::new(metrics);

    let cfg = tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
        max_send_queue: Some(1),
        max_message_size: Some(64 * 1024),
        max_frame_size: Some(128 * 1024),
        accept_unmasked_frames: false,
    };

    let mut ws = tokio_tungstenite::accept_async_with_config(socket, Some(cfg)).await?;

    m!(metrics.ws_connections.inc());

    let mut last_activity = Instant::now();
    let mut penultimate_activity: Instant;

    let mut pinger = tokio::time::interval(Duration::new(PING_SECONDS, 0));
    pinger.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            biased;
            _ = pinger.tick() => {
                let now = Instant::now();
                let delta = now.saturating_duration_since(last_activity);
                if delta > Duration::from_secs(PING_TIMEOUT) {
                    m!(metrics.ping_timeouts.inc());
                    return Ok(());
                }
                if delta > Duration::from_secs(PING_SECONDS) {
                    m!(metrics.pings_sent.inc());
                    ws.send(Message::Ping(vec![])).await?;
                }
            }
            msg = ws.next() => {
                penultimate_activity = last_activity;
                last_activity = Instant::now();
                #[cfg(feature = "metrics")]
                if let Some(Ok(ref msg))=  msg {
                    client_session_metrics.msgs += 1;
                    match msg {
                        Message::Text(x) => {
                            metrics.text_messages.inc();
                            metrics.bytes_from_clients.inc_by(x.len() as u64);
                            client_session_metrics.bytes += x.len() as u64;
                        },
                        Message::Binary(x) => {
                            metrics.binary_messages.inc();
                            metrics.bytes_from_clients.inc_by(x.len() as u64);
                            client_session_metrics.bytes += x.len() as u64;
                        },
                        Message::Ping(x) => {
                            metrics.ping_messages.inc();
                            metrics.bytes_from_clients.inc_by(x.len() as u64);
                            client_session_metrics.bytes += x.len() as u64;
                        },
                        Message::Pong(_) => metrics.pong_messages.inc(),
                        Message::Close(_) => metrics.close_messages.inc(),
                        Message::Frame(_) => metrics.other_messages.inc(),
                    }
                }
                match msg {
                    None | Some(Err(..)) => return Ok(()),
                    Some(Ok(m @ (Message::Text(..) | Message::Binary(..)))) => {
                        ws.send(m).await?;
                        if last_activity.saturating_duration_since(penultimate_activity) < Duration::from_millis(20) {
                            tokio::task::yield_now().await;
                        }
                    },
                    _ => (),
                }
            }
        };
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), anyhow::Error> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 || args[1].starts_with('-') {
        println!("Usage: wsmirror <tcp_bind_socket_address>");
        return Ok(());
    }
    let bindaddr: SocketAddr = args[1].parse()?;
    let listener = TcpListener::bind(bindaddr).await?;

    #[cfg(feature = "metrics")]
    let metrics: &metrics::Metrics = Box::leak(Box::new(metrics::new()));

    macro_rules! m {
        ($e:expr) => {
            #[cfg(feature = "metrics")]
            $e
        };
    }

    #[cfg(feature = "metrics")]
    if let Ok(prometheus_addr) = std::env::var("PROMETHEUS_EXPORT_ADDR") {
        let prometheus_addr: SocketAddr = prometheus_addr.parse()?;
        prometheus_exporter::start(prometheus_addr)?;
    }

    loop {
        let (mut socket, _) = listener.accept().await?;

        if CLIENT_COUNTER.load(std::sync::atomic::Ordering::SeqCst) >= MAX_CLIENTS + 10 {
            drop(socket);
            m!(metrics.rejected_quick.inc());
            continue;
        }

        CLIENT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        m!(metrics.connections.inc());

        tokio::spawn(async move {
            if CLIENT_COUNTER.load(std::sync::atomic::Ordering::SeqCst) >= MAX_CLIENTS {
                let _1 = socket
                    .write_all(b"HTTP/1.0 503 Too Many Clients Connected\r\n\r\n")
                    .await;
                let _2 = socket.shutdown().await;
                CLIENT_COUNTER.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                m!(metrics.rejected_overload.inc());
                return;
            }

            let _1 = serve_client(
                socket,
                #[cfg(feature = "metrics")] metrics,
            ).await;

            CLIENT_COUNTER.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            m!(metrics.disconnections.inc());
        });
    }
}
