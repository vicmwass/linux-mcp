//! Identity consumer: validate a token, then revoke it and observe it fail.
//! Run: `cargo run -p linux-mcp-contracts --example identity_flow --features test-support`

#[cfg(feature = "test-support")]
fn main() {
    use linux_mcp_contracts::backend::services::{BearerToken, RevocationTarget};
    use linux_mcp_contracts::test_support::{memory::block_on, scenarios::demo};

    let d = demo();
    let identity = d.world.services().identity;
    let call = d.call_ctx();
    let token = BearerToken::new(d.alice.mcp_token.clone());
    let principal = block_on(identity.validate_mcp_token(&call, token.clone())).unwrap();
    println!("authenticated {principal:?}");
    let ctx = d.agent_ctx(&d.alice);
    block_on(identity.revoke(
        &ctx,
        RevocationTarget::Session(ctx.principal.session().clone()),
    ))
    .unwrap();
    let after = block_on(identity.validate_mcp_token(&call, token)).unwrap_err();
    println!("after revocation: {after}");
}

#[cfg(not(feature = "test-support"))]
fn main() {
    eprintln!("this example needs --features test-support");
}
