use hickory_resolver::proto::rr::RData;
use hickory_resolver::{Resolver, TokioResolver};
use std::net::IpAddr;
use std::sync::OnceLock;
use std::time::Duration;

static RESOLVER: OnceLock<TokioResolver> = OnceLock::new();

fn resolver() -> &'static TokioResolver {
    RESOLVER.get_or_init(|| {
        let mut builder = Resolver::builder_tokio()
            .expect("failed to read system DNS config (/etc/resolv.conf)");

        builder.options_mut().timeout = Duration::from_millis(500);

        builder.build().expect("failed to build DNS resolver")
    })
}

/// Attempts a reverse DNS (PTR) lookup for `addr`. Returns `None` on any
/// failure — no PTR record, timeout, or a response that doesn't carry a
/// PTR record in its answers.
pub async fn reverse_lookup(addr: IpAddr) -> Option<String> {
    let response = resolver().reverse_lookup(addr).await.ok()?;

    let name = response.answers().iter().find_map(|record| match &record.data {
        RData::PTR(ptr) => Some(ptr.0.to_string()),
        _ => None,
    })?;

    Some(name.trim_end_matches('.').to_string())
}