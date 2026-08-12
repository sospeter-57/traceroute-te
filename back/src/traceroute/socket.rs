use anyhow::{Context, Result};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

pub struct IcmpSocket {
    inner: Socket,
}

impl IcmpSocket {
    pub fn new() -> Result<Self> {
        let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))
            .context("failed to create raw ICMP socket — need root or CAP_NET_RAW")?;

        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .context("failed to set socket read timeout")?;

        Ok(Self { inner: socket })
    }

    pub fn set_ttl(&self, ttl: u8) -> Result<()> {
        self.inner
            .set_ttl(ttl as u32)
            .context("failed to set TTL on socket")
    }

    pub fn send_to(&self, packet: &[u8], target: IpAddr) -> Result<()> {
        let addr = SocketAddr::new(target, 0);
        self.inner
            .send_to(packet, &SockAddr::from(addr))
            .context("failed to send ICMP packet")?;
        Ok(())
    }

    pub fn recv(&self) -> Result<Option<(Vec<u8>, IpAddr)>> {
        let mut buf = [std::mem::MaybeUninit::uninit(); 512];

        match self.inner.recv_from(&mut buf) {
            Ok((len, from)) => {
                let bytes: Vec<u8> = buf[..len]
                    .iter()
                    .map(|b| unsafe { b.assume_init() })
                    .collect();
                let ip = from.as_socket().map(|s| s.ip()).context("no ip in from addr")?;
                Ok(Some((bytes, ip)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(None),
            Err(e) => Err(e).context("failed to receive from socket"),
        }
    }
}