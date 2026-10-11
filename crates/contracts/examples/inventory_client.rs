//! Inventory consumer: create, edit with a revision, hit a stale-edit conflict, delete.
//! Run: `cargo run -p linux-mcp-contracts --example inventory_client --features test-support`

#[cfg(feature = "test-support")]
fn main() {
    use linux_mcp_contracts::public::inventory::*;
    use linux_mcp_contracts::test_support::{memory::block_on, scenarios::demo};

    let d = demo();
    let services = d.world.services();
    let ctx = d.browser_ctx(&d.alice);
    let request: CreateServerRequest = serde_json::from_str(
        r#"{"display_name":"example","hostname":"example.internal","username":"deploy","environment":"testing"}"#,
    )
    .unwrap();
    let created = block_on(services.inventory.create(&ctx, request)).unwrap();
    println!("created {} at revision {}", created.id, created.revision);
    let edit = UpdateServerRequest {
        expected_revision: created.revision,
        display_name: Some(DisplayName::new("renamed").unwrap()),
        hostname: None,
        port: None,
        username: None,
        environment: None,
        enabled: None,
        approval_mode: None,
    };
    let updated = block_on(services.inventory.update(&ctx, created.id, edit.clone())).unwrap();
    println!("updated to revision {}", updated.revision);
    let stale = block_on(services.inventory.update(&ctx, created.id, edit)).unwrap_err();
    println!("stale edit rejected: {stale}");
    block_on(
        services
            .inventory
            .delete(&ctx, created.id, updated.revision),
    )
    .unwrap();
    println!("deleted");
}

#[cfg(not(feature = "test-support"))]
fn main() {
    eprintln!("this example needs --features test-support");
}
