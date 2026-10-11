//! Governance consumer: approval request, human decision, single execution, owner-scoped audit.
//! Run: `cargo run -p linux-mcp-contracts --example governance_flow --features test-support`

#[cfg(feature = "test-support")]
fn main() {
    use linux_mcp_contracts::public::api::ApprovalDecision;
    use linux_mcp_contracts::public::api::AuditQuery;
    use linux_mcp_contracts::public::operations::OperationRequest;
    use linux_mcp_contracts::test_support::memory::block_on;
    use linux_mcp_contracts::test_support::scenarios::{Driven, demo};
    use serde_json::json;

    let d = demo();
    let services = d.world.services();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let request: OperationRequest = serde_json::from_value(json!({
        "server_id": d.servers.alice_shared, "tool": "delete_user", "tool_schema_version": 1,
        "arguments": {"username": "olduser", "remove_home": true, "confirm": true}
    }))
    .unwrap();
    let Driven::ApprovalRequired(approval) = block_on(driver.run(&agent, request)).unwrap() else {
        unreachable!("delete_user needs approval")
    };
    println!(
        "pending approval {} for {}",
        approval.approval_id,
        approval.intent.tool()
    );
    let human = d.browser_ctx(&d.alice);
    block_on(driver.decide(&human, &approval, ApprovalDecision::Approve)).unwrap();
    let result = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap();
    println!("executed once: {:?}", result.outcome);
    let again = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap_err();
    println!("reuse rejected: {again}");
    let mine = block_on(services.audit.list_owned(&agent, AuditQuery::default())).unwrap();
    let bobs = block_on(
        services
            .audit
            .list_owned(&d.agent_ctx(&d.bob), AuditQuery::default()),
    )
    .unwrap();
    println!(
        "{} audit records for Alice; Bob sees {}",
        mine.items.len(),
        bobs.items.len()
    );
}

#[cfg(not(feature = "test-support"))]
fn main() {
    eprintln!("this example needs --features test-support");
}
