mod dns;
mod hop;
mod packet;
mod socket;
pub mod ws;

pub use hop::{Hop, HopEvent, HopUpdate};

use anyhow::{Context, Result};
use socket::IcmpSocket;
use std::net::IpAddr;
use std::process;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};

pub struct Options {
    pub max_hops: u8,
    pub identifier: u16,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_hops: 30,
            identifier: (process::id() & 0xFFFF) as u16,
        }
    }
}

/// Runs a traceroute to `target`, streaming events as they happen rather
/// than collecting everything before returning. The receiver yields a
/// `Hop` as soon as each TTL resolves, and a `HostnameResolved` update
/// later if/when that hop's reverse DNS lookup completes.
pub fn run(target: IpAddr, opts: Options) -> mpsc::Receiver<HopEvent> {
    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(async move {
        if let Err(e) = run_inner(target, opts, tx).await {
            eprintln!("traceroute failed: {e:#}");
        }
    });

    rx
}

async fn run_inner(target: IpAddr, opts: Options, tx: mpsc::Sender<HopEvent>) -> Result<()> {
    let sock = tokio::task::spawn_blocking(IcmpSocket::new)
        .await
        .context("socket setup task panicked")??;
    let sock = Arc::new(Mutex::new(sock));

    for ttl in 1..=opts.max_hops {
        let hop = send_and_receive(sock.clone(), target, ttl, opts.identifier).await?;

        let reached = hop.reached_target;
        let addr_to_resolve = hop.addr;

        if tx.send(HopEvent::Hop(hop)).await.is_err() {
            // Receiver dropped (client disconnected) — stop probing.
            return Ok(());
        }

        if let Some(addr) = addr_to_resolve {
            let tx = tx.clone();
            tokio::spawn(async move {
                if let Some(hostname) = dns::reverse_lookup(addr).await {
                    let _ = tx
                        .send(HopEvent::HostnameResolved(HopUpdate { ttl, hostname }))
                        .await;
                }
            });
        }

        if reached {
            break;
        }
    }

    Ok(())
}

/// Runs one TTL's set_ttl+send+recv as a single blocking task. The mutex
/// is held only inside the blocking closure — locked, used synchronously,
/// dropped before the next .await — so it's never held across an await
/// point.
async fn send_and_receive(
    sock: Arc<Mutex<IcmpSocket>>,
    target: IpAddr,
    ttl: u8,
    identifier: u16,
) -> Result<Hop> {
    tokio::task::spawn_blocking(move || {
        let sock = sock.blocking_lock();

        sock.set_ttl(ttl)?;

        let probe = packet::build_echo_request(identifier, ttl as u16);
        let sent_at = Instant::now();
        sock.send_to(&probe, target)?;

        let result = sock.recv()?;

        Ok(match result {
            None => Hop::timeout(ttl),
            Some((bytes, from)) => {
                let rtt = sent_at.elapsed();
                match packet::parse_icmp_response(&bytes) {
                    Some(packet::IcmpResponse::TimeExceeded) => Hop {
                        ttl,
                        addr: Some(from),
                        rtt: Some(rtt),
                        reached_target: false,
                        hostname: None,
                    },
                    Some(packet::IcmpResponse::EchoReply) => Hop {
                        ttl,
                        addr: Some(from),
                        rtt: Some(rtt),
                        reached_target: true,
                        hostname: None,
                    },
                    _ => Hop::timeout(ttl),
                }
            }
        })
    })
    .await
    .context("send/recv task panicked")?
}
