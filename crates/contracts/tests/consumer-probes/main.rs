//! Step 00-11: consumer and security probes.
//!
//! Each module below is a minimal *consumer* (inventory, identity, executor, MCP adapter,
//! governance, dashboard/public) written only against the contract surface and the in-memory
//! mocks. Passing these shows the contract is usable and that the documented call sequence
//! yields the documented outcomes. **They do not prove production isolation**: the mocks
//! simulate behavior; a real database, OAuth validation and SSH are not exercised.
//!
//! Real compile checks (not simulations): `public_consumer_builds_on_host` and
//! `public_consumer_builds_for_wasm32` compile a standalone public-feature crate. The wasm test
//! fails (it does not skip) when `wasm32-unknown-unknown` is not installed.
//! `LINUX_MCP_SKIP_PROBES=1` skips only the host cargo-spawning probe.

mod adapter;

use std::path::{Path, PathBuf};
use std::process::Command;

use adapter::{McpAdapter, Reply, render_text};
use linux_mcp_contracts::backend::credentials::CredentialPurpose;
use linux_mcp_contracts::backend::identity::Scope;
use linux_mcp_contracts::backend::services::*;
use linux_mcp_contracts::public::api::*;
use linux_mcp_contracts::public::errors::{ApprovalErrorReason, ErrorCode, ErrorDetails};
use linux_mcp_contracts::public::ids::*;
use linux_mcp_contracts::public::inventory::*;
use linux_mcp_contracts::public::mcp::*;
use linux_mcp_contracts::public::operations::*;
use linux_mcp_contracts::public::pagination::{Page, PageRequest};
use linux_mcp_contracts::test_support::memory::{ExecScript, block_on};
use linux_mcp_contracts::test_support::scenarios::*;
use serde_json::{Value, json};

fn adapter_for(d: &Demo, actor: &Actor) -> McpAdapter {
    McpAdapter {
        driver: d.driver(),
        ctx: d.agent_ctx(actor),
    }
}

fn call(a: &McpAdapter, tool: &str, args: Value) -> (ToolResultBody, bool) {
    match a.call(tool, args) {
        Reply::Result { envelope, is_error } => {
            // Every structured result must also round-trip as v1 JSON.
            let wire = serde_json::to_value(&envelope).unwrap();
            let back: ToolResultEnvelope = serde_json::from_value(wire).unwrap();
            assert_eq!(back, envelope);
            assert_eq!(envelope.is_error(), is_error);
            (envelope.result, is_error)
        }
        Reply::ProtocolError { .. } => panic!("unexpected protocol error"),
    }
}

fn server_arg(id: ServerId) -> Value {
    json!(id.to_string())
}

// ---------------------------------------------------------------------------------------
// Consumer: inventory (dashboard backend)
// ---------------------------------------------------------------------------------------

#[test]
fn inventory_consumer_crud_with_revisions() {
    let d = demo();
    let s = d.world.services();
    let ctx = d.browser_ctx(&d.alice);
    let created = block_on(
        s.inventory.create(
            &ctx,
            serde_json::from_value(json!({
                "display_name": "probe", "hostname": "probe.example.internal",
                "username": "deploy", "environment": "testing"
            }))
            .unwrap(),
        ),
    )
    .unwrap();
    assert_eq!(created.revision, ServerRevision::INITIAL);
    let edit = UpdateServerRequest {
        expected_revision: created.revision,
        display_name: Some(DisplayName::new("renamed").unwrap()),
        hostname: None,
        port: None,
        username: None,
        environment: None,
        enabled: Some(false),
        approval_mode: None,
    };
    let updated = block_on(s.inventory.update(&ctx, created.id, edit.clone())).unwrap();
    assert_eq!(updated.revision.get(), 2);
    assert!(!updated.enabled);
    // A stale edit conflicts and reports the current revision.
    let err = block_on(s.inventory.update(&ctx, created.id, edit)).unwrap_err();
    assert_eq!(err.code(), ErrorCode::RevisionConflict);
    // A disabled server no longer resolves for operations.
    let agent = d.agent_ctx(&d.alice);
    let err = block_on(s.targets.resolve(&agent, created.id, Scope::ServersInspect)).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Validation);
    block_on(s.inventory.delete(&ctx, created.id, updated.revision)).unwrap();
    assert_eq!(
        block_on(s.inventory.get_owned(&ctx, created.id))
            .unwrap_err()
            .code(),
        ErrorCode::NotFound
    );
}

// ---------------------------------------------------------------------------------------
// Consumer: identity (MCP service / dashboard)
// ---------------------------------------------------------------------------------------

#[test]
fn identity_consumer_revocation_and_wrong_credentials() {
    let d = demo();
    let s = d.world.services();
    let call_ctx = d.call_ctx();
    let alice = d.agent_ctx(&d.alice);
    assert!(block_on(s.identity.revalidate(&alice)).is_ok());
    block_on(s.identity.revoke(
        &alice,
        RevocationTarget::Session(alice.principal.session().clone()),
    ))
    .unwrap();
    // Revoked identity: existing context, fresh validation and every service call fail.
    for err in [
        block_on(s.identity.revalidate(&alice)).unwrap_err(),
        block_on(
            s.identity
                .validate_mcp_token(&call_ctx, BearerToken::new(d.alice.mcp_token.clone())),
        )
        .unwrap_err(),
        block_on(s.inventory.list_owned(&alice, PageRequest::default())).unwrap_err(),
    ] {
        assert_eq!(err.code(), ErrorCode::Unauthenticated);
    }
    // Alice's browser session (different session) is independently revocable.
    let browser = d.browser_ctx(&d.alice);
    assert!(block_on(s.identity.revalidate(&browser)).is_ok());
    // Bob is unaffected.
    assert!(
        block_on(
            s.inventory
                .list_owned(&d.agent_ctx(&d.bob), PageRequest::default())
        )
        .is_ok()
    );
}

// ---------------------------------------------------------------------------------------
// Consumer: executor + MCP adapter
// ---------------------------------------------------------------------------------------

#[test]
fn mcp_adapter_maps_every_outcome_distinctly() {
    let d = demo();
    let a = adapter_for(&d, &d.alice);
    let server = d.servers.alice_shared;

    // Success.
    let (body, is_error) = call(
        &a,
        "execute_command",
        json!({"server_id": server_arg(server), "command": "id"}),
    );
    assert!(!is_error);
    assert!(matches!(
        body,
        ToolResultBody::Outcome {
            outcome: OperationOutcome::Succeeded { .. },
            ..
        }
    ));

    // Policy refusal: an outcome, not a protocol error.
    let (body, is_error) = call(
        &a,
        "execute_command",
        json!({"server_id": server_arg(server), "command": "rm -rf /"}),
    );
    assert!(is_error);
    assert!(matches!(
        body,
        ToolResultBody::Outcome {
            outcome: OperationOutcome::Refused {
                reason: RefusalReason::PolicyDenied,
                ..
            },
            ..
        }
    ));

    // Approval required, even when the agent sets confirm: true.
    let delete = json!({"server_id": server_arg(server), "username": "olduser", "confirm": true, "remove_home": true});
    let (body, is_error) = call(&a, "delete_user", delete);
    assert!(!is_error);
    let ToolResultBody::ApprovalRequired(approval) = body else {
        panic!("{body:?}")
    };
    assert_eq!(approval.status, ApprovalStatus::Pending);
    assert!(approval.dashboard_path.starts_with("/approvals/"));
    assert_eq!(
        d.world.attempts().len(),
        1,
        "only the first command ran; delete_user did not"
    );

    // Timeout before submission, then disconnect after submission.
    d.world.script(server, ExecScript::TimeoutBeforeSubmit);
    let (body, _) = call(
        &a,
        "execute_command",
        json!({"server_id": server_arg(server), "command": "id"}),
    );
    assert!(matches!(
        body,
        ToolResultBody::Outcome {
            outcome: OperationOutcome::FailedBeforeSubmission { .. },
            submission: SubmissionState::NotSubmitted,
            ..
        }
    ));
    d.world.script(server, ExecScript::DisconnectAfterSubmit);
    let (body, is_error) = call(
        &a,
        "execute_command",
        json!({"server_id": server_arg(server), "command": "id"}),
    );
    assert!(is_error);
    assert!(matches!(
        body,
        ToolResultBody::Outcome {
            outcome: OperationOutcome::UnknownAfterSubmission { .. },
            ..
        }
    ));
    let attempts_after_unknown = d.world.attempts().len();
    assert_eq!(attempts_after_unknown, 2);

    // Unknown tools are protocol errors; missing arguments are tool-level errors.
    assert!(matches!(
        a.call("no_such_tool", json!({})),
        Reply::ProtocolError { code: -32602, .. }
    ));
    let (body, is_error) = call(&a, "get_user", json!({"username": "x"}));
    assert!(is_error);
    assert!(
        matches!(body, ToolResultBody::Error { error } if error.code() == ErrorCode::Validation)
    );
    // A server ID that is not a UUID is a validation error, not a lookup.
    let (_, is_error) = call(
        &a,
        "get_user",
        json!({"server_id": "alice-prod", "username": "x"}),
    );
    assert!(is_error);
    // Text rendering distinguishes unknown from failure.
    let text = render_text(&a.call(
        "execute_command",
        json!({"server_id": server_arg(server), "command": "id"}),
    ));
    assert!(
        text.contains("unknown") && text.contains("will not be retried"),
        "{text}"
    );
}

#[test]
fn mcp_adapter_isolates_alice_and_bob() {
    let d = demo();
    let alice = adapter_for(&d, &d.alice);
    let bob = adapter_for(&d, &d.bob);
    // Alice names Bob's server ID directly, bypassing discovery: indistinguishable from absent.
    let (foreign, _) = call(
        &alice,
        "execute_command",
        json!({"server_id": server_arg(d.servers.bob_shared), "command": "id"}),
    );
    let absent = ServerId::parse("5e000000-0000-4000-8000-0000000000ee").unwrap();
    let (missing, _) = call(
        &alice,
        "execute_command",
        json!({"server_id": server_arg(absent), "command": "id"}),
    );
    assert_eq!(
        serde_json::to_string(&foreign).unwrap(),
        serde_json::to_string(&missing).unwrap()
    );
    assert!(d.world.attempts().is_empty());
    // Bob can run on his own server; his attempt carries his credential, not Alice's.
    call(
        &bob,
        "execute_command",
        json!({"server_id": server_arg(d.servers.bob_shared), "command": "id"}),
    );
    assert_eq!(d.world.attempts()[0].credential_id, d.bob_ssh_key.id);
}

// ---------------------------------------------------------------------------------------
// Consumer: governance (approvals + audit) and security scenarios
// ---------------------------------------------------------------------------------------

fn request_approval(_d: &Demo, a: &McpAdapter, server: ServerId) -> ApprovalRequiredResult {
    let (body, _) = call(
        a,
        "delete_user",
        json!({"server_id": server_arg(server), "username": "olduser", "remove_home": true, "confirm": true}),
    );
    match body {
        ToolResultBody::ApprovalRequired(r) => r,
        other => panic!("{other:?}"),
    }
}

#[test]
fn governance_consumer_decides_and_audit_is_owner_scoped() {
    let d = demo();
    let s = d.world.services();
    let a = adapter_for(&d, &d.alice);
    let approval = request_approval(&d, &a, d.servers.alice_shared);
    let human = d.browser_ctx(&d.alice);

    let pending = block_on(s.approvals.list(
        &human,
        Some(ApprovalStatus::Pending),
        PageRequest::default(),
    ))
    .unwrap();
    assert_eq!(pending.items.len(), 1);
    // The browser shows the stored intent, not agent-provided text.
    let shown = &pending.items[0];
    assert_eq!(shown.intent.tool().as_str(), "delete_user");
    assert_eq!(shown.intent.arguments().value()["username"], "olduser");
    assert_eq!(shown.intent_digest, approval.intent_digest);

    let decided = block_on(s.approvals.decide(
        &human,
        approval.approval_id,
        ApprovalDecisionRequest {
            decision: ApprovalDecision::Approve,
            intent_digest: approval.intent_digest.clone(),
        },
    ))
    .unwrap();
    assert_eq!(decided.status, ApprovalStatus::Approved);

    // Audit is owner scoped: Bob sees none of Alice's records.
    let bob = d.agent_ctx(&d.bob);
    assert!(
        block_on(s.audit.list_owned(&bob, AuditQuery::default()))
            .unwrap()
            .items
            .is_empty()
    );
    let mine = block_on(
        s.audit
            .list_owned(&d.agent_ctx(&d.alice), AuditQuery::default()),
    )
    .unwrap();
    assert!(
        mine.items
            .iter()
            .any(|r| r.action == AuditAction::ApprovalRequested)
    );
    assert!(
        mine.items
            .iter()
            .any(|r| r.action == AuditAction::ApprovalDecided)
    );
    assert_eq!(
        block_on(s.approvals.get(&bob, approval.approval_id))
            .unwrap_err()
            .code(),
        ErrorCode::NotFound
    );
}

#[test]
fn stale_target_blocks_approved_operation() {
    let d = demo();
    let s = d.world.services();
    let a = adapter_for(&d, &d.alice);
    let server = d.servers.alice_shared;
    let approval = request_approval(&d, &a, server);
    let human = d.browser_ctx(&d.alice);
    block_on(s.approvals.decide(
        &human,
        approval.approval_id,
        ApprovalDecisionRequest {
            decision: ApprovalDecision::Approve,
            intent_digest: approval.intent_digest.clone(),
        },
    ))
    .unwrap();
    // The owner rotates the host's credential after approving: revision moves on.
    let rotate: PutCredentialRequest = serde_json::from_value(json!({
        "expected_revision": 2,
        "credential": {"type": "sudo_password", "password": "FAKE-SECRET-new-sudo"}
    }))
    .unwrap();
    block_on(s.credentials.put(&human, server, rotate)).unwrap();
    let err = block_on(a.driver.execute_approved(&a.ctx, approval.approval_id)).unwrap_err();
    assert!(matches!(
        err.details(),
        Some(ErrorDetails::ApprovalRequired {
            reason: ApprovalErrorReason::BindingMismatch,
            ..
        })
    ));
    assert_eq!(d.world.attempts().len(), 0);
}

#[test]
fn changed_arguments_cannot_reuse_an_approval() {
    let d = demo();
    let s = d.world.services();
    let a = adapter_for(&d, &d.alice);
    let server = d.servers.alice_shared;
    let approval = request_approval(&d, &a, server);
    let human = d.browser_ctx(&d.alice);
    block_on(s.approvals.decide(
        &human,
        approval.approval_id,
        ApprovalDecisionRequest {
            decision: ApprovalDecision::Approve,
            intent_digest: approval.intent_digest.clone(),
        },
    ))
    .unwrap();

    // Build a different intent (deleting root's data instead) and try to consume the approval.
    let target = block_on(s.targets.resolve(&a.ctx, server, Scope::ServersMutate)).unwrap();
    let now = d.world.now();
    let altered = OperationIntent::new(IntentParts {
        operation_id: d.world.next_operation_id(),
        installation_id: a.ctx.principal.installation_id(),
        owner: d.alice.user,
        server_id: server,
        server_revision: target.revision(),
        tool: ToolId::new("delete_user").unwrap(),
        tool_schema_version: ToolSchemaVersion::V1,
        arguments: CanonicalArguments::new(
            json!({"username": "root", "remove_home": true, "confirm": true}),
        )
        .unwrap(),
        risk: RiskLevel::High,
        created_at: now,
        expires_at: now.checked_add_millis(60_000).unwrap(),
    })
    .unwrap();
    let policy = block_on(s.executor.evaluate(&a.ctx, &altered, &target)).unwrap();
    let err =
        block_on(
            s.approvals
                .consume(&a.ctx, approval.approval_id, &altered, &target, &policy),
        )
        .unwrap_err();
    assert!(matches!(
        err.details(),
        Some(ErrorDetails::ApprovalRequired {
            reason: ApprovalErrorReason::BindingMismatch,
            ..
        })
    ));
    assert!(d.world.attempts().is_empty());
    // The failed attempt did not burn the approval: the original intent still executes once.
    let result = block_on(a.driver.execute_approved(&a.ctx, approval.approval_id)).unwrap();
    assert!(matches!(result.outcome, OperationOutcome::Succeeded { .. }));
    assert_eq!(d.world.attempts().len(), 1);
}

#[test]
fn expired_and_reused_approvals_fail_and_unknown_outcome_is_final() {
    let d = demo();
    let s = d.world.services();
    let a = adapter_for(&d, &d.alice);
    let server = d.servers.alice_shared;
    let human = d.browser_ctx(&d.alice);

    // Reuse after an unknown outcome: consumed, never retried.
    d.world.script(server, ExecScript::DisconnectAfterSubmit);
    let first = request_approval(&d, &a, server);
    block_on(s.approvals.decide(
        &human,
        first.approval_id,
        ApprovalDecisionRequest {
            decision: ApprovalDecision::Approve,
            intent_digest: first.intent_digest.clone(),
        },
    ))
    .unwrap();
    let result = block_on(a.driver.execute_approved(&a.ctx, first.approval_id)).unwrap();
    assert!(matches!(
        result.outcome,
        OperationOutcome::UnknownAfterSubmission { .. }
    ));
    let retry = block_on(a.driver.execute_approved(&a.ctx, first.approval_id)).unwrap_err();
    assert!(matches!(
        retry.details(),
        Some(ErrorDetails::ApprovalRequired {
            reason: ApprovalErrorReason::Consumed,
            ..
        })
    ));
    assert_eq!(d.world.attempts().len(), 1);

    // Expiry.
    let second = request_approval(&d, &a, server);
    block_on(s.approvals.decide(
        &human,
        second.approval_id,
        ApprovalDecisionRequest {
            decision: ApprovalDecision::Approve,
            intent_digest: second.intent_digest.clone(),
        },
    ))
    .unwrap();
    d.world.clock().advance_secs(20 * 60);
    let fresh = McpAdapter {
        driver: d.driver(),
        ctx: d.agent_ctx(&d.alice),
    };
    let err = block_on(
        fresh
            .driver
            .execute_approved(&fresh.ctx, second.approval_id),
    )
    .unwrap_err();
    assert!(matches!(
        err.details(),
        Some(ErrorDetails::ApprovalRequired {
            reason: ApprovalErrorReason::Expired,
            ..
        })
    ));
    assert_eq!(d.world.attempts().len(), 1);
}

#[test]
fn secrets_never_appear_in_results_audit_errors_or_debug_output() {
    let d = demo();
    let s = d.world.services();
    let a = adapter_for(&d, &d.alice);
    let server = d.servers.alice_shared;
    let mut transcript = String::new();

    for (tool, args) in [
        (
            "execute_command",
            json!({"server_id": server_arg(server), "command": "id"}),
        ),
        (
            "delete_user",
            json!({"server_id": server_arg(server), "username": "olduser", "remove_home": true, "confirm": true}),
        ),
        (
            "execute_command",
            json!({"server_id": server_arg(d.servers.bob_shared), "command": "id"}),
        ),
    ] {
        let reply = a.call(tool, args);
        transcript.push_str(&format!("{reply:?}\n{}\n", render_text(&reply)));
    }
    // Secret-named arguments are rejected before they can reach intent, audit or logs.
    let reply = a.call("execute_command", json!({"server_id": server_arg(server), "command": "id", "password": "FAKE-SECRET-hunter2"}));
    assert!(matches!(&reply, Reply::Result { is_error: true, .. }));
    transcript.push_str(&format!("{reply:?}"));

    let target = block_on(s.targets.resolve(&a.ctx, server, Scope::ServersInspect)).unwrap();
    let lease = block_on(s.credentials.resolve_for(
        &a.ctx,
        &target,
        CredentialPurpose::SshAuthentication,
    ))
    .unwrap();
    transcript.push_str(&format!(
        "{lease:?}\n{target:?}\n{:?}\n{:?}",
        a.ctx,
        d.world.attempts()
    ));
    let audit = block_on(
        s.audit
            .list_owned(&d.agent_ctx(&d.alice), AuditQuery::default()),
    )
    .unwrap();
    transcript.push_str(&serde_json::to_string(&audit).unwrap());
    let summaries = block_on(s.inventory.list_owned(&a.ctx, PageRequest::default())).unwrap();
    transcript.push_str(&serde_json::to_string(&summaries).unwrap());
    let token_text = format!("{:?}", BearerToken::new(d.alice.mcp_token.clone()));
    transcript.push_str(&token_text);

    for forbidden in [
        "FAKE-SECRET",
        "hunter2",
        d.alice.mcp_token.as_str(),
        d.alice.session_cookie.as_str(),
    ] {
        assert!(!transcript.contains(forbidden), "leaked {forbidden}");
    }
    assert!(!d.world.audit_contains_secret_marker());
}

// ---------------------------------------------------------------------------------------
// Consumer: dashboard / public-only (real compile checks)
// ---------------------------------------------------------------------------------------

#[test]
fn dashboard_view_models_use_only_public_json() {
    let d = demo();
    let s = d.world.services();
    let page = block_on(
        s.inventory
            .list_owned(&d.agent_ctx(&d.alice), PageRequest::default()),
    )
    .unwrap();
    let body = serde_json::to_string(&page).unwrap();
    let parsed: Page<ServerSummary> = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed, page);
    // The same JSON is all a browser needs; it carries no credential content or owner.
    for forbidden in ["\"owner", "FAKE-SECRET", "\"private_key\"", "\"password\""] {
        assert!(!body.contains(forbidden), "{forbidden}");
    }
}

fn probe_crate(name: &str) -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir.parent().unwrap().parent().unwrap();
    let probe = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let src = crate_dir.join("tests/consumer-probes/public-consumer");
    std::fs::create_dir_all(probe.join("src")).unwrap();
    std::fs::copy(src.join("src/lib.rs"), probe.join("src/lib.rs")).unwrap();
    let manifest = std::fs::read_to_string(src.join("Cargo.toml"))
        .unwrap()
        .replace(
            "path = \"../../..\"",
            &format!(
                "path = {:?}",
                crate_dir.display().to_string().replace('\\', "/")
            ),
        );
    std::fs::write(probe.join("Cargo.toml"), manifest).unwrap();
    std::fs::copy(workspace.join("Cargo.lock"), probe.join("Cargo.lock")).unwrap();
    probe
}

fn cargo_check(probe: &Path, extra: &[&str]) -> std::process::Output {
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["check", "--offline", "--quiet"])
        .args(extra)
        .current_dir(probe)
        .env("CARGO_TARGET_DIR", probe.join("target"))
        .output()
        .unwrap()
}

#[test]
fn public_consumer_builds_on_host() {
    if std::env::var_os("LINUX_MCP_SKIP_PROBES").is_some() {
        eprintln!("skipped by LINUX_MCP_SKIP_PROBES");
        return;
    }
    let probe = probe_crate("public-consumer-host");
    let out = cargo_check(&probe, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Required release validation: fails (never skips) when the wasm target is missing.
#[test]
fn public_consumer_builds_for_wasm32() {
    let sysroot = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    let sysroot = PathBuf::from(String::from_utf8(sysroot.stdout).unwrap().trim());
    let installed = sysroot.join("lib/rustlib/wasm32-unknown-unknown").is_dir();
    assert!(
        installed,
        "wasm32-unknown-unknown is not installed: run `rustup target add wasm32-unknown-unknown`. \
         Step 00-11 cannot pass without this build."
    );
    let probe = probe_crate("public-consumer-wasm");
    let out = cargo_check(&probe, &["--target", "wasm32-unknown-unknown"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn public_consumer_logic_runs_against_real_payloads() {
    // Exercise the same helper logic natively by compiling it into this test (include!): it
    // only uses the public prelude, so it behaves identically in the browser build.
    mod consumer {
        include!("public-consumer/src/lib.rs");
    }
    let d = demo();
    let s = d.world.services();
    let ctx = d.agent_ctx(&d.alice);
    let page = block_on(s.inventory.list_owned(&ctx, PageRequest::default())).unwrap();
    let parsed = consumer::parse_server_page(&serde_json::to_string(&page).unwrap()).unwrap();
    let rows: Vec<String> = parsed.items.iter().map(consumer::server_row).collect();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].contains("Alice shared host") && rows[0].contains("Verified"));
    assert!(rows[1].contains("Unenrolled"));

    let body =
        consumer::build_create_request("New box", "NEW.example.org", 2222, "deploy").unwrap();
    let created: CreateServerRequest = serde_json::from_str(&body).unwrap();
    assert_eq!(created.hostname.as_str(), "new.example.org");
    let bad = consumer::build_create_request("", "bad host", 0, "Root").unwrap_err();
    // Only the first invalid field is reported by this form helper; it never echoes values.
    assert_eq!(bad.violations[0].field.as_str(), "display_name");

    let conflict = serde_json::to_string(&ErrorEnvelope {
        error: linux_mcp_contracts::public::errors::PublicError::revision_conflict(
            d.call_ctx().request_id,
            ServerRevision::new(5).unwrap(),
        ),
    })
    .unwrap();
    assert!(consumer::is_revision_conflict(&conflict));
    assert_eq!(consumer::error_banner(&conflict).unwrap().0, 409);

    let a = adapter_for(&d, &d.alice);
    request_approval(&d, &a, d.servers.alice_shared);
    let approvals = block_on(s.approvals.list(&ctx, None, PageRequest::default())).unwrap();
    let rows = consumer::approval_rows(&serde_json::to_string(&approvals).unwrap()).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].contains("delete_user") && rows[0].contains("High"));
    assert_eq!(consumer::contract_versions(), ("1.0.0", "lmcp-canon-v1"));
}
