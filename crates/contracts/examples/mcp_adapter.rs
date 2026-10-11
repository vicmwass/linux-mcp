//! MCP adapter consumer: legacy-shaped tool calls mapped onto the shared contracts, returning
//! the `linux-mcp/v1` structured result. Mock only; no transport.
//! Run: `cargo run -p linux-mcp-contracts --example mcp_adapter --features test-support`

#[cfg(feature = "test-support")]
#[allow(dead_code)]
#[path = "../tests/consumer-probes/adapter.rs"]
mod adapter;

#[cfg(feature = "test-support")]
fn main() {
    use adapter::{McpAdapter, render_text};
    use linux_mcp_contracts::test_support::scenarios::demo;
    use serde_json::json;

    let d = demo();
    let a = McpAdapter {
        driver: d.driver(),
        ctx: d.agent_ctx(&d.alice),
    };
    let server = d.servers.alice_shared.to_string();
    for (tool, args) in [
        (
            "execute_command",
            json!({"server_id": server, "command": "id"}),
        ),
        (
            "execute_command",
            json!({"server_id": server, "command": "rm -rf /"}),
        ),
        (
            "delete_user",
            json!({"server_id": server, "username": "olduser", "confirm": true, "remove_home": true}),
        ),
        ("no_such_tool", json!({})),
    ] {
        let reply = a.call(tool, args);
        println!("{tool}: {}", render_text(&reply));
    }
}

#[cfg(not(feature = "test-support"))]
fn main() {
    eprintln!("this example needs --features test-support");
}
