//! Dashboard (browser-side) consumer using only public contract types and JSON.
//! Works with `--no-default-features --features public`.
//! Run: `cargo run -p linux-mcp-contracts --example dashboard_public --no-default-features --features public`

use linux_mcp_contracts::prelude::*;

fn main() {
    let body = r#"{"items":[{"id":"3f2b8c1e-5d4a-4b6e-9f10-2a3b4c5d6e7f","display_name":"Build box",
      "hostname":"build-01.example.internal","port":22,"username":"deploy","environment":"development",
      "revision":3,"enabled":true,"approval_mode":"medium_and_above","host_trust":{"status":"verified","fingerprint":"SHA256:x","verified_at":"2026-10-01T12:00:00.000Z"},
      "credentials":[],"created_at":"2026-09-30T08:00:00.000Z","updated_at":"2026-10-01T12:00:00.000Z"}],"next_cursor":null}"#;
    let page: Page<ServerSummary> = serde_json::from_str(body).unwrap();
    for server in &page.items {
        println!(
            "{} {}:{} rev {} trust {:?}",
            server.display_name,
            server.hostname,
            server.port.get(),
            server.revision,
            server.host_trust.status
        );
    }
    println!(
        "contract {} / canonicalization {}",
        linux_mcp_contracts::CONTRACT_VERSION,
        CANONICALIZATION_VERSION
    );
}
