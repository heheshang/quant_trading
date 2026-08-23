//! Lightweight Prometheus `/metrics` HTTP endpoint.
//!
//! Serves the `monitor-layer` registry in the Prometheus text exposition
//! format over a bare `tokio::net::TcpListener` (no axum/warp dependency).
//! We read the request line and only handle `GET /metrics`; everything else
//! is answered with `404 Not Found`.

use monitor_layer::handle_metrics_request;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};

/// Bind and serve `/metrics` on `port`. Runs forever.
pub async fn run(port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind Prometheus listener on {}: {}", addr, e);
            return;
        }
    };

    info!("Prometheus metrics endpoint listening on http://{}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, peer)) => {
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket).await {
                        warn!("Prometheus connection {} failed: {}", peer, e);
                    }
                });
            }
            Err(e) => {
                warn!("Prometheus accept error: {}", e);
            }
        }
    }
}

async fn handle_connection(mut socket: TcpStream) -> std::io::Result<()> {
    let mut buf = vec![0u8; 2048];
    let n = socket.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let response = handle_metrics_request(&buf[..n]);
    socket.write_all(response.as_bytes()).await?;
    socket.shutdown().await
}
