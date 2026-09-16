//! The plugin serves what the compiled-in sources served, and uses only what
//! a call brings. No network: Describe and Connected are answered from the
//! sources' own declarations, and a fetch with no keys is refused before any
//! request is made.

use std::sync::Arc;
use std::time::Duration;

use arvo_alpaca::{Alpaca, Keys};
use arvo_data::source::{Credential, Source};
use arvo_data::BarInterval;
use arvo_plugin_host::source::v1::source_client::SourceClient;
use arvo_plugin_host::source::v1::Grant;
use arvo_plugin_host::source::{no_grants, serve, Granter, GrpcSource, Served};

const ADDRESS: &str = "http://127.0.0.1:50082";

fn compiled_in() -> Vec<Box<dyn Source>> {
    vec![
        Box::new(Alpaca::iex()),
        Box::new(Alpaca::sip()),
        Box::new(Alpaca::iex_total_return()),
        Box::new(Alpaca::sip_total_return()),
    ]
}

async fn start() {
    let addr: std::net::SocketAddr = "127.0.0.1:50082".parse().expect("an address");
    let plugin = Served::with_grants("alpaca", "Alpaca", "0.0.0", |grant| {
        let keys = grant
            .filter(|grant| !grant.key_id.is_empty())
            .map(|grant| Keys { key_id: grant.key_id.clone(), secret: grant.secret.clone() });
        vec![
            Box::new(Alpaca::iex().with_keys(keys.clone())),
            Box::new(Alpaca::sip().with_keys(keys.clone())),
            Box::new(Alpaca::iex_total_return().with_keys(keys.clone())),
            Box::new(Alpaca::sip_total_return().with_keys(keys)),
        ]
    });
    tokio::spawn(serve(addr, plugin));
    for _ in 0..50 {
        if SourceClient::connect(ADDRESS.to_owned()).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the plugin never came up");
}

#[tokio::test]
async fn all_four_alpaca_sources_cross_with_their_declarations_and_keys_decide_the_session() {
    start().await;
    let day = chrono::NaiveDate::from_ymd_opt(2026, 1, 30).expect("a date");

    // Nothing in the host's keychain: four sources, none connected, and a
    // fetch is refused as no session before any request leaves the machine.
    let bare = GrpcSource::discover_with(ADDRESS, no_grants()).await.expect("discovered");
    let compiled = compiled_in();
    assert_eq!(bare.len(), compiled.len());
    for (served, own) in bare.iter().zip(&compiled) {
        assert_eq!(served.id(), own.id());
        assert_eq!(served.venue(), own.venue());
        assert_eq!(served.basis(), own.basis(), "{}: the declaration the boundary exists to keep", own.id());
        assert_eq!(served.credential(), Credential::Keys);
        assert!(!served.connected().await.expect("asked"), "{}: no keys, not connected", own.id());
    }
    let refused = bare[0].bars("SPY", BarInterval::DAILY, day, day).await.expect_err("no keys, no call");
    assert!(refused.needs_sign_in(), "{refused:?}");

    // A key pair for this vendor: it crosses with each call and every source
    // is connected for that call. (No fetch here; that would be a real request.)
    let granter: Granter = Arc::new(|vendor| {
        (vendor == "alpaca").then(|| Grant { key_id: "k".into(), secret: "s".into(), bearer: String::new() })
    });
    let keyed = GrpcSource::discover_with(ADDRESS, granter).await.expect("discovered");
    for served in &keyed {
        assert!(served.connected().await.expect("asked"), "{}: keys given, connected", served.id());
    }
}
