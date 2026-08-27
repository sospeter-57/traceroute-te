use serde::Serialize;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct Hop {
    pub ttl: u8,
    pub addr: Option<IpAddr>,
    #[serde(serialize_with = "serialize_rtt_ms")]
    pub rtt: Option<Duration>,
    pub reached_target: bool,
    pub hostname: Option<String>,
}

impl Hop {
    pub fn timeout(ttl: u8) -> Self {
        Self {
            ttl,
            addr: None,
            rtt: None,
            reached_target: false,
            hostname: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HopUpdate {
    pub ttl: u8,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HopEvent {
    Hop(Hop),
    HostnameResolved(HopUpdate),
}

/// `Duration` serializes as a verbose `{secs, nanos}` object by default —
/// not what a frontend wants. Send milliseconds as a plain number instead,
/// since that's the unit real traceroute output uses.
fn serialize_rtt_ms<S>(rtt: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match rtt {
        Some(d) => serializer.serialize_f64(d.as_secs_f64() * 1000.0),
        None => serializer.serialize_none(),
    }
}