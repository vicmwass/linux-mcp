//! Executor consumer: the reference call sequence, including a refusal and an unknown outcome.
//! Run: `cargo run -p linux-mcp-contracts --example executor_flow --features test-support`

#[cfg(feature = "test-support")]
fn main() {
    use linux_mcp_contracts::public::operations::OperationRequest;
    use linux_mcp_contracts::test_support::memory::{ExecScript, block_on};
    use linux_mcp_contracts::test_support::scenarios::demo;
    use serde_json::json;

    let d = demo();
    let ctx = d.agent_ctx(&d.alice);
    let driver = d.driver();
    let run = |command: &str| {
        let request: OperationRequest = serde_json::from_value(json!({
            "server_id": d.servers.alice_shared, "tool": "execute_command",
            "tool_schema_version": 1, "arguments": {"command": command}
        }))
        .unwrap();
        block_on(driver.run(&ctx, request)).unwrap()
    };
    println!("read-only:  {:?}", run("id"));
    println!("refused:    {:?}", run("rm -rf /"));
    d.world
        .script(d.servers.alice_shared, ExecScript::DisconnectAfterSubmit);
    println!("disconnect: {:?}", run("id"));
    println!("remote attempts recorded: {}", d.world.attempts().len());
}

#[cfg(not(feature = "test-support"))]
fn main() {
    eprintln!("this example needs --features test-support");
}
