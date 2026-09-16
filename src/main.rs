//! Alpaca as an Arvo plugin: the second source to live outside the app, and
//! the one that shows the credential pattern (ADR-0022 point 4).
//!
//! Everything Alpaca-specific stays in `arvo-alpaca`, inside `arvo-desktop`;
//! this is the process boundary and nothing else. Four sources are served,
//! because a feed and an adjustment are each part of a dataset's identity:
//! IEX and SIP, each split-adjusted and total return, under four venues.
//!
//! The keys arrive with each call. Arvo reads them from its keychain and sends
//! what one call needs; the sources here are built from exactly that and
//! never look past it, so a call that brought no keys is a call with no
//! session whatever this machine holds. Nothing is kept between calls.
//!
//! The address comes from `ARVO_PLUGIN_ADDR`. Arvo's supervisor (ADR-0023)
//! will set it; until then a person does, and the default is a fixed port.

use std::net::SocketAddr;

use arvo_alpaca::{Alpaca, Keys};
use arvo_plugin_host::source::v1::Grant;
use arvo_plugin_host::source::{serve, Served, SERVICE};

const DEFAULT_ADDR: &str = "127.0.0.1:50053";

/// The key pair a call brought, if it brought one worth the name.
fn keys_from(grant: Option<&Grant>) -> Option<Keys> {
    grant
        .filter(|grant| !grant.key_id.is_empty() && !grant.secret.is_empty())
        .map(|grant| Keys { key_id: grant.key_id.clone(), secret: grant.secret.clone() })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = std::env::var("ARVO_PLUGIN_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_owned()).parse()?;
    let plugin = Served::with_grants("alpaca", "Alpaca", env!("CARGO_PKG_VERSION"), |grant| {
        let keys = keys_from(grant);
        vec![
            Box::new(Alpaca::iex().with_keys(keys.clone())),
            Box::new(Alpaca::sip().with_keys(keys.clone())),
            Box::new(Alpaca::iex_total_return().with_keys(keys.clone())),
            Box::new(Alpaca::sip_total_return().with_keys(keys)),
        ]
    });
    println!("arvo-plugin-alpaca serving {SERVICE} at {addr}");
    serve(addr, plugin).await?;
    Ok(())
}
