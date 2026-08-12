use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::net::{IpAddr, ToSocketAddrs};

use crate::traceroute::{self, HopEvent, Options};

#[derive(Debug, Deserialize)]
pub struct TracerouteQuery {
    target: String,
}

/// Entry point axum calls on GET /ws/traceroute?target=<host-or-ip>.
/// Validates the target *before* upgrading to a WebSocket, so a bad
/// hostname gets a normal HTTP error instead of a socket that opens
/// and immediately closes with no explanation.
pub async fn handler(
    ws: WebSocketUpgrade,
    Query(query): Query<TracerouteQuery>,
) -> impl IntoResponse {
    match resolve(&query.target) {
        Ok(target) => ws.on_upgrade(move |socket| run_traceroute(socket, target)),
        Err(e) => {
            // Can't upgrade and then report an error over HTTP — once
            // upgraded, everything has to happen over the WS protocol.
            // A plain 400 here is far easier for the frontend to handle
            // than a WS connection that opens and instantly errors.
            (
                axum::http::StatusCode::BAD_REQUEST,
                format!("could not resolve target: {e}"),
            )
                .into_response()
        }
    }
}

fn resolve(input: &str) -> anyhow::Result<IpAddr> {
    if let Ok(ip) = input.parse::<IpAddr>() {
        return Ok(ip);
    }

    let mut addrs = (input.as_ref(), 0)
        .to_socket_addrs()
        .map_err(|e| anyhow::anyhow!("failed to resolve host {input}: {e}"))?;

    addrs
        .next()
        .map(|addr| addr.ip())
        .ok_or_else(|| anyhow::anyhow!("no addresses found for host: {input}"))
}

async fn run_traceroute(mut socket: WebSocket, target: IpAddr) {
    let mut rx = traceroute::run(target, Options::default());

    while let Some(event) = rx.recv().await {
        let json = match serde_json::to_string(&event) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("failed to serialize HopEvent: {e}");
                continue;
            }
        };

        if socket.send(Message::Text(json)).await.is_err() {
            // Client disconnected mid-run — stop pushing, the sender
            // side of `rx` will notice on its next send and the
            // traceroute task will wind down on its own.
            return;
        }
    }

    // Traceroute finished (reached target, hit max_hops, or errored
    // internally) — close cleanly so the client knows the run is done.
    let _ = socket.send(Message::Close(None)).await;
}
