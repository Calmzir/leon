//! A relay for development: `cargo run -p leon-mobile --example dev-relay`.
//!
//! The relay Zavu operates is not part of this repository. To try a phone
//! against a computer on the same network, start this one, point the
//! computer at it (`leon host --pair --relay ws://<this computer>:8787`, or
//! the Relay server setting) and give the phone the same address.
//!
//! It is the test double of `leon-link`: a plain forwarder with no TLS, no
//! limits and no accounts. What it forwards is end-to-end encrypted all the
//! same, but it is not a service to expose to the internet.

use std::net::SocketAddr;

use leon_link::test_support::TestRelay;

#[tokio::main]
async fn main() {
    let listen = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8787".to_owned());
    let addr: SocketAddr = match listen.parse() {
        Ok(addr) => addr,
        Err(_) => {
            eprintln!("usage: dev-relay [address:port]   (default 0.0.0.0:8787)");
            std::process::exit(2);
        }
    };
    let relay = TestRelay::start_on(addr).await;
    println!("Development relay listening on {}", relay.url());
    println!("Not for the internet: no TLS, no limits. Ctrl-C stops it.");
    let _ = tokio::signal::ctrl_c().await;
    relay.stop().await;
}
