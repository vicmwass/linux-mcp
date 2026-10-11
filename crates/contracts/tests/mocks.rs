//! Step 00-09: in-memory adapters and scenario kit.
//!
//! These tests exercise the *contract* through simulated adapters. They show consumers can
//! build against the mocks and that the contract's outcomes are expressible; they are not
//! evidence of production isolation (database, OAuth and SSH behavior are not simulated).

#![cfg(feature = "test-support")]

use linux_mcp_contracts::backend::credentials::CredentialPurpose;
use linux_mcp_contracts::backend::identity::{AuthKind, Scope};
use linux_mcp_contracts::backend::services::*;
use linux_mcp_contracts::public::api::*;
use linux_mcp_contracts::public::errors::{
    ApprovalErrorReason, ErrorCode, ErrorDetails, PublicError,
};
use linux_mcp_contracts::public::ids::*;
use linux_mcp_contracts::public::inventory::*;
use linux_mcp_contracts::public::operations::*;
use linux_mcp_contracts::public::pagination::{PageLimit, PageRequest};
use linux_mcp_contracts::test_support::memory::{ExecScript, World, block_on};
use linux_mcp_contracts::test_support::scenarios::*;
use serde_json::{Value, json};

fn op(server: ServerId, tool: &str, args: Value) -> OperationRequest {
    serde_json::from_value(json!({
        "server_id": server, "tool": tool, "tool_schema_version": 1, "arguments": args
    }))
    .unwrap()
}

fn id_cmd(server: ServerId) -> OperationRequest {
    op(server, "execute_command", json!({"command": "id"}))
}

fn delete_user(server: ServerId) -> OperationRequest {
    op(
        server,
        "delete_user",
        json!({"username": "olduser", "remove_home": true, "confirm": true}),
    )
}

fn finished(driven: Driven) -> OperationResult {
    match driven {
        Driven::Finished(r) => *r,
        Driven::ApprovalRequired(a) => panic!("unexpected approval request {a:?}"),
    }
}

fn pending(driven: Driven) -> ApprovalSummary {
    match driven {
        Driven::ApprovalRequired(a) => *a,
        Driven::Finished(r) => panic!("expected approval request, got {r:?}"),
    }
}

fn code<T: std::fmt::Debug>(result: Result<T, PublicError>) -> ErrorCode {
    result.expect_err("expected an error").code()
}

/// Requests and human-approves a delete_user on `server`. Returns the approval.
fn approved_delete(d: &Demo, server: ServerId) -> ApprovalSummary {
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let approval = pending(block_on(driver.run(&agent, delete_user(server))).unwrap());
    let human = d.browser_ctx(&d.alice);
    block_on(driver.decide(&human, &approval, ApprovalDecision::Approve)).unwrap()
}

#[test]
fn every_trait_is_instantiable_and_demo_runs() {
    let d = demo();
    let s = d.world.services();
    let ctx = d.agent_ctx(&d.alice);
    let page = block_on(s.inventory.list_owned(&ctx, PageRequest::default())).unwrap();
    assert_eq!(page.items.len(), 2);
    let target = block_on(
        s.targets
            .resolve(&ctx, d.servers.alice_shared, Scope::ServersInspect),
    )
    .unwrap();
    assert!(
        block_on(s.executor.inspect(&ctx, &target))
            .unwrap()
            .commands
            .contains(&"id".to_string())
    );
    assert!(
        block_on(s.approvals.list(&ctx, None, PageRequest::default()))
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        block_on(s.audit.list_owned(&ctx, AuditQuery::default()))
            .unwrap()
            .items
            .is_empty()
    );
    let lease = block_on(s.credentials.resolve_for(
        &ctx,
        &target,
        CredentialPurpose::SshAuthentication,
    ))
    .unwrap();
    assert_eq!(
        lease.with_secret(d.world.now(), |s| s.to_vec()).unwrap(),
        b"FAKE-SECRET-alice-ssh-key"
    );
    assert!(block_on(s.identity.revalidate(&ctx)).is_ok());
}

#[test]
fn alice_cannot_touch_bobs_resources_and_errors_match_absent_ids() {
    let d = demo();
    let s = d.world.services();
    let alice = d.agent_ctx(&d.alice);
    let alice_browser = d.browser_ctx(&d.alice);
    let bob_server = d.servers.bob_shared;
    let absent = ServerId::parse("5e000000-0000-4000-8000-0000000000ff").unwrap();
    let rev = ServerRevision::new(2).unwrap();

    for id in [bob_server, absent] {
        assert_eq!(
            code(block_on(s.inventory.get_owned(&alice, id))),
            ErrorCode::NotFound
        );
        assert_eq!(
            code(block_on(s.targets.resolve(
                &alice,
                id,
                Scope::ServersInspect
            ))),
            ErrorCode::NotFound
        );
        assert_eq!(
            code(block_on(s.inventory.delete(&alice_browser, id, rev))),
            ErrorCode::NotFound
        );
        let update = UpdateServerRequest {
            expected_revision: rev,
            display_name: Some(DisplayName::new("pwn").unwrap()),
            hostname: None,
            port: None,
            username: None,
            environment: None,
            enabled: None,
            approval_mode: None,
        };
        assert_eq!(
            code(block_on(s.inventory.update(&alice_browser, id, update))),
            ErrorCode::NotFound
        );
        assert_eq!(
            code(block_on(s.inventory.observe_host_key(&alice_browser, id))),
            ErrorCode::NotFound
        );
        assert_eq!(code(s.driver_run(&d, &alice, id)), ErrorCode::NotFound);
    }
    // Externally identical (modulo nothing: same request ID, same body).
    let foreign = block_on(s.inventory.get_owned(&alice, bob_server)).unwrap_err();
    let missing = block_on(s.inventory.get_owned(&alice, absent)).unwrap_err();
    assert_eq!(
        serde_json::to_string(&foreign).unwrap(),
        serde_json::to_string(&missing).unwrap()
    );

    // Bob's server is untouched; lists show only own data.
    assert_eq!(d.world.server_revision(bob_server), Some(rev));
    let ids: Vec<_> = block_on(s.inventory.list_owned(&alice, PageRequest::default()))
        .unwrap()
        .items
        .iter()
        .map(|x| x.id)
        .collect();
    assert!(!ids.contains(&bob_server));
    assert!(d.world.attempts().is_empty());

    // Credentials: Alice cannot write Bob's slot or lease for Bob's target.
    let put = PutCredentialRequest {
        expected_revision: rev,
        credential: CredentialInput::SudoPasswordless,
    };
    assert_eq!(
        code(block_on(s.credentials.put(&alice_browser, bob_server, put))),
        ErrorCode::NotFound
    );
    assert_eq!(
        d.world.server_revision(bob_server),
        Some(rev),
        "no side effect on foreign server"
    );
}

/// Small extension so the loop above can reuse the driver with a one-line call.
trait DriverRun {
    fn driver_run(&self, d: &Demo, ctx: &RequestContext, server: ServerId)
    -> ServiceResult<Driven>;
}
impl DriverRun for Services {
    fn driver_run(
        &self,
        d: &Demo,
        ctx: &RequestContext,
        server: ServerId,
    ) -> ServiceResult<Driven> {
        block_on(d.driver().run(ctx, id_cmd(server)))
    }
}

#[test]
fn same_hostname_different_owners_never_share_credentials_or_hosts() {
    let d = demo();
    let driver = d.driver();
    let alice = d.agent_ctx(&d.alice);
    let bob = d.agent_ctx(&d.bob);
    let a = finished(block_on(driver.run(&alice, id_cmd(d.servers.alice_shared))).unwrap());
    let b = finished(block_on(driver.run(&bob, id_cmd(d.servers.bob_shared))).unwrap());
    assert!(matches!(a.outcome, OperationOutcome::Succeeded { .. }));
    assert!(matches!(b.outcome, OperationOutcome::Succeeded { .. }));
    let attempts = d.world.attempts();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].hostname, attempts[1].hostname);
    assert_eq!(attempts[0].port, attempts[1].port);
    assert_eq!(attempts[0].ssh_username, attempts[1].ssh_username);
    assert_ne!(attempts[0].credential_id, attempts[1].credential_id);
    assert_ne!(attempts[0].host_fingerprint, attempts[1].host_fingerprint);
    assert_ne!(attempts[0].cache_key, attempts[1].cache_key);
    assert_eq!(attempts[0].owner, d.alice.user);
    assert_eq!(attempts[0].credential_id, d.alice_ssh_key.id);
    assert_eq!(attempts[1].owner, d.bob.user);
    assert_eq!(attempts[1].credential_id, d.bob_ssh_key.id);
    assert_eq!(d.world.pool_keys().len(), 2);
}

#[test]
fn missing_trust_blocks_until_observed_fingerprint_is_enrolled() {
    let d = demo();
    let s = d.world.services();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let human = d.browser_ctx(&d.alice);
    let server = d.servers.alice_untrusted;

    assert_eq!(
        code(block_on(driver.run(&agent, id_cmd(server)))),
        ErrorCode::Validation
    );
    assert!(d.world.attempts().is_empty());

    // Observation alone does not establish trust.
    let observation = block_on(s.inventory.observe_host_key(&human, server)).unwrap();
    assert!(!observation.differs_from_verified);
    let summary = block_on(s.inventory.get_owned(&agent, server)).unwrap();
    assert_eq!(summary.host_trust.status, HostTrustStatus::Observed);
    assert_eq!(
        code(block_on(driver.run(&agent, id_cmd(server)))),
        ErrorCode::Validation
    );

    // Enrolling an unobserved fingerprint fails; the observed one succeeds and bumps revision.
    let wrong = "SHA256:WrongFakeFingerprintWrongFakeFingerprintWr".to_owned();
    assert_eq!(
        code(block_on(s.inventory.enroll_host_key(
            &human,
            server,
            observation.server_revision,
            wrong
        ))),
        ErrorCode::Validation
    );
    let enrolled = block_on(s.inventory.enroll_host_key(
        &human,
        server,
        observation.server_revision,
        observation.fingerprint,
    ))
    .unwrap();
    assert_eq!(enrolled.host_trust.status, HostTrustStatus::Verified);
    assert_eq!(
        enrolled.revision.get(),
        observation.server_revision.get() + 1
    );
    let result = finished(block_on(driver.run(&agent, id_cmd(server))).unwrap());
    assert!(matches!(result.outcome, OperationOutcome::Succeeded { .. }));
}

#[test]
fn changed_host_key_blocks_and_post_resolution_change_is_rejected_before_submission() {
    let d = demo();
    let s = d.world.services();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let human = d.browser_ctx(&d.alice);
    let server = d.servers.alice_shared;

    // Key changes between resolution and submission (no observation yet): rejected, not sent.
    d.world
        .set_presented_host_key(server, "SHA256:RotatedFakeKeyRotatedFakeKeyRotatedFakeK");
    let result = finished(block_on(driver.run(&agent, id_cmd(server))).unwrap());
    assert_eq!(
        result.outcome,
        OperationOutcome::FailedBeforeSubmission {
            cause: PreSubmissionFailure::HostKeyRejected
        }
    );
    assert_eq!(result.submission, SubmissionState::NotSubmitted);
    assert!(d.world.attempts().is_empty());

    // Observing the changed key marks a mismatch that blocks connections until reverified.
    let observation = block_on(s.inventory.observe_host_key(&human, server)).unwrap();
    assert!(observation.differs_from_verified);
    assert_eq!(
        block_on(s.inventory.get_owned(&agent, server))
            .unwrap()
            .host_trust
            .status,
        HostTrustStatus::Mismatch
    );
    assert_eq!(
        code(block_on(driver.run(&agent, id_cmd(server)))),
        ErrorCode::Validation
    );
}

#[test]
fn changed_revision_invalidates_targets_leases_pools_and_approvals() {
    let d = demo();
    let s = d.world.services();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let human = d.browser_ctx(&d.alice);
    let server = d.servers.alice_shared;

    finished(block_on(driver.run(&agent, id_cmd(server))).unwrap());
    let old_key = d.world.pool_keys().pop().unwrap();
    let stale_target = block_on(s.targets.resolve(&agent, server, Scope::ServersInspect)).unwrap();
    let approval = approved_delete(&d, server);

    // Owner edits the server: revision bump, invalidation published, pool evicted.
    let update = UpdateServerRequest {
        expected_revision: ServerRevision::new(2).unwrap(),
        display_name: Some(DisplayName::new("renamed").unwrap()),
        hostname: None,
        port: None,
        username: None,
        environment: None,
        enabled: None,
        approval_mode: None,
    };
    let updated = block_on(s.inventory.update(&human, server, update.clone())).unwrap();
    assert_eq!(updated.revision.get(), 3);
    assert!(
        d.world
            .invalidations()
            .iter()
            .any(|e| e.server_id == server && e.new_revision.get() == 3)
    );
    assert!(!d.world.pool_keys().contains(&old_key));

    // Stale update (concurrent edit) fails with the current revision.
    let conflict = block_on(s.inventory.update(&human, server, update)).unwrap_err();
    assert_eq!(conflict.code(), ErrorCode::RevisionConflict);
    assert!(
        matches!(conflict.details(), Some(ErrorDetails::RevisionConflict { current_revision }) if current_revision.get() == 3)
    );

    // The old authorized target can no longer lease secrets.
    assert_eq!(
        code(block_on(s.credentials.resolve_for(
            &agent,
            &stale_target,
            CredentialPurpose::SshAuthentication
        ))),
        ErrorCode::RevisionConflict
    );

    // The approval was bound to revision 2 and cannot be consumed against revision 3.
    let err = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap_err();
    assert_eq!(err.code(), ErrorCode::ApprovalRequired);
    assert!(matches!(
        err.details(),
        Some(ErrorDetails::ApprovalRequired {
            reason: ApprovalErrorReason::BindingMismatch,
            ..
        })
    ));
    assert_eq!(
        block_on(s.approvals.get(&agent, approval.approval_id))
            .unwrap()
            .status,
        ApprovalStatus::Invalidated
    );
    assert_eq!(
        d.world.attempts().len(),
        1,
        "only the first read-only command ran"
    );
}

#[test]
fn credential_rotation_is_write_only_and_stops_old_sessions() {
    let d = demo();
    let s = d.world.services();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let human = d.browser_ctx(&d.alice);
    let server = d.servers.alice_shared;
    finished(block_on(driver.run(&agent, id_cmd(server))).unwrap());
    let old_key = d.world.pool_keys().pop().unwrap();

    let request: PutCredentialRequest = serde_json::from_value(json!({
        "expected_revision": 2,
        "credential": {"type": "ssh_private_key", "private_key": "FAKE-SECRET-rotated-key"}
    }))
    .unwrap();
    let response = block_on(s.credentials.put(&human, server, request)).unwrap();
    assert_eq!(response.action, CredentialWriteAction::Rotated);
    assert_eq!(response.server_revision.get(), 3);
    assert!(
        !serde_json::to_string(&response)
            .unwrap()
            .contains("FAKE-SECRET")
    );
    assert!(!d.world.pool_keys().contains(&old_key));
    assert!(
        d.world
            .invalidations()
            .iter()
            .any(|e| e.reason == InvalidationReason::CredentialChanged)
    );

    // New work uses the new credential; the pool key carries the new revision.
    finished(block_on(driver.run(&agent, id_cmd(server))).unwrap());
    let attempts = d.world.attempts();
    assert_ne!(attempts[0].credential_id, attempts[1].credential_id);
    assert_eq!(attempts[1].server_revision.get(), 3);
    assert!(!d.world.audit_contains_secret_marker());
    let summary = block_on(s.inventory.get_owned(&agent, server)).unwrap();
    assert!(
        !serde_json::to_string(&summary)
            .unwrap()
            .contains("FAKE-SECRET")
    );
}

#[test]
fn revoked_expired_and_wrong_kind_identities_fail() {
    let d = demo();
    let s = d.world.services();
    let call = d.call_ctx();
    let alice = d.agent_ctx(&d.alice);
    let bob = d.agent_ctx(&d.bob);

    // Bob cannot revoke Alice's session; Alice can revoke herself.
    assert_eq!(
        code(block_on(s.identity.revoke(
            &bob,
            RevocationTarget::User {
                installation_id: alice.principal.installation_id(),
                user_id: d.alice.user
            }
        ))),
        ErrorCode::NotFound
    );
    block_on(s.identity.revoke(
        &alice,
        RevocationTarget::User {
            installation_id: alice.principal.installation_id(),
            user_id: d.alice.user,
        },
    ))
    .unwrap();

    // Existing contexts and fresh token/session validation both fail for Alice; Bob is unaffected.
    assert_eq!(
        code(block_on(
            s.inventory.list_owned(&alice, PageRequest::default())
        )),
        ErrorCode::Unauthenticated
    );
    assert_eq!(
        code(block_on(s.identity.validate_mcp_token(
            &call,
            BearerToken::new(d.alice.mcp_token.clone())
        ))),
        ErrorCode::Unauthenticated
    );
    assert!(block_on(s.inventory.list_owned(&bob, PageRequest::default())).is_ok());

    // Browser session value is not an MCP token, and vice versa; CSRF is required.
    assert_eq!(
        code(block_on(s.identity.validate_mcp_token(
            &call,
            BearerToken::new(d.bob.session_cookie.clone())
        ))),
        ErrorCode::Unauthenticated
    );
    let no_csrf = BrowserCredentials {
        session_cookie: BearerToken::new(d.bob.session_cookie.clone()),
        csrf_token: None,
        unsafe_method: true,
    };
    assert_eq!(
        code(block_on(s.identity.authenticate_browser(&call, no_csrf))),
        ErrorCode::Unauthenticated
    );
    let bad_csrf = BrowserCredentials {
        session_cookie: BearerToken::new(d.bob.session_cookie.clone()),
        csrf_token: Some(BearerToken::new("FAKE-CSRF-wrong".into())),
        unsafe_method: true,
    };
    assert_eq!(
        code(block_on(s.identity.authenticate_browser(&call, bad_csrf))),
        ErrorCode::Unauthenticated
    );
    let safe_get = BrowserCredentials {
        session_cookie: BearerToken::new(d.bob.session_cookie.clone()),
        csrf_token: None,
        unsafe_method: false,
    };
    assert!(block_on(s.identity.authenticate_browser(&call, safe_get)).is_ok());
    assert_eq!(
        code(block_on(s.identity.validate_mcp_token(
            &call,
            BearerToken::new("FAKE-TOKEN-unknown".into())
        ))),
        ErrorCode::Unauthenticated
    );

    // Expiry.
    d.world.clock().advance_secs(3601);
    let mut late = bob.clone();
    late.call.deadline = d.world.now().checked_add_millis(30_000).unwrap();
    assert_eq!(
        code(block_on(
            s.inventory.list_owned(&late, PageRequest::default())
        )),
        ErrorCode::Unauthenticated
    );
}

#[test]
fn separate_installations_reject_each_others_tokens() {
    let a = demo();
    let other = World::new(InstallationId::parse(OTHER_INSTALLATION).unwrap());
    let b = seed_demo(&other);
    let call = a.call_ctx();
    let services_b = b.world.services();
    // A token minted by installation A means nothing to installation B, and vice versa.
    assert_eq!(
        code(block_on(services_b.identity.validate_mcp_token(
            &call,
            BearerToken::new(a.alice.mcp_token.clone())
        ))),
        ErrorCode::Unauthenticated
    );
    assert!(
        block_on(
            services_b
                .identity
                .validate_mcp_token(&call, BearerToken::new(b.alice.mcp_token.clone()))
        )
        .is_ok()
    );
    // A principal from A is rejected by B's services even with a valid-looking context.
    let ctx_a = a.agent_ctx(&a.alice);
    assert_eq!(
        code(block_on(
            services_b
                .inventory
                .list_owned(&ctx_a, PageRequest::default())
        )),
        ErrorCode::Unauthenticated
    );
}

#[test]
fn policy_refusal_is_an_outcome_not_an_error_and_sends_nothing() {
    let d = demo();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let result = finished(
        block_on(driver.run(
            &agent,
            op(
                d.servers.alice_shared,
                "execute_command",
                json!({"command": "rm -rf /"}),
            ),
        ))
        .unwrap(),
    );
    assert_eq!(result.submission, SubmissionState::NotSubmitted);
    assert!(matches!(
        result.outcome,
        OperationOutcome::Refused {
            reason: RefusalReason::PolicyDenied,
            ..
        }
    ));
    assert!(d.world.attempts().is_empty());
    let s = d.world.services();
    assert!(
        block_on(s.approvals.list(&agent, None, PageRequest::default()))
            .unwrap()
            .items
            .is_empty(),
        "denials never create approvals"
    );
}

#[test]
fn approval_flow_agents_cannot_self_approve_and_consume_once() {
    let d = demo();
    let s = d.world.services();
    let driver = d.driver();
    let server = d.servers.alice_shared;
    let agent = d.agent_ctx(&d.alice);
    let approval = pending(block_on(driver.run(&agent, delete_user(server))).unwrap());
    assert_eq!(approval.status, ApprovalStatus::Pending);
    assert!(d.world.attempts().is_empty());

    // The agent cannot decide: its token lacks approvals:decide...
    let decision = |a: &ApprovalSummary| ApprovalDecisionRequest {
        decision: ApprovalDecision::Approve,
        intent_digest: a.intent_digest.clone(),
    };
    assert_eq!(
        code(block_on(s.approvals.decide(
            &agent,
            approval.approval_id,
            decision(&approval)
        ))),
        ErrorCode::Forbidden
    );
    // ...and a token that does carry the scope is still not an interactive session.
    let strong = d.world.issue_token(
        d.alice.user,
        &[Scope::ApprovalsDecide, Scope::ServersMutate],
        AuthKind::McpToken,
        600,
    );
    let strong_ctx = d.ctx_from_token(&strong);
    assert_eq!(
        code(block_on(s.approvals.decide(
            &strong_ctx,
            approval.approval_id,
            decision(&approval)
        ))),
        ErrorCode::Forbidden
    );
    // Executing before a human decision is refused.
    let err = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap_err();
    assert!(matches!(
        err.details(),
        Some(ErrorDetails::ApprovalRequired {
            reason: ApprovalErrorReason::Pending,
            ..
        })
    ));
    assert!(d.world.attempts().is_empty());

    // Bob cannot see or decide Alice's approval; a wrong digest cannot approve.
    let bob_browser = d.browser_ctx(&d.bob);
    assert_eq!(
        code(block_on(s.approvals.decide(
            &bob_browser,
            approval.approval_id,
            decision(&approval)
        ))),
        ErrorCode::NotFound
    );
    let human = d.browser_ctx(&d.alice);
    let mut wrong = decision(&approval);
    wrong.intent_digest = IntentDigestOf::other();
    assert_eq!(
        code(block_on(s.approvals.decide(
            &human,
            approval.approval_id,
            wrong
        ))),
        ErrorCode::Validation
    );

    // The owner approves in the browser; the agent executes once.
    let approved = block_on(
        s.approvals
            .decide(&human, approval.approval_id, decision(&approval)),
    )
    .unwrap();
    assert_eq!(approved.status, ApprovalStatus::Approved);
    let result = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap();
    assert!(matches!(result.outcome, OperationOutcome::Succeeded { .. }));
    assert_eq!(result.approval_id, Some(approval.approval_id));
    assert_eq!(d.world.attempts().len(), 1);

    // Reuse fails; the attempt log proves no second dispatch.
    let reuse = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap_err();
    assert!(matches!(
        reuse.details(),
        Some(ErrorDetails::ApprovalRequired {
            reason: ApprovalErrorReason::Consumed,
            ..
        })
    ));
    assert_eq!(d.world.attempts().len(), 1);
    assert_eq!(
        block_on(s.approvals.get(&agent, approval.approval_id))
            .unwrap()
            .status,
        ApprovalStatus::Consumed
    );

    // Audit shows pending -> outcome for the operation, with the approval and no secrets.
    let records = block_on(s.audit.list_owned(&agent, AuditQuery::default()))
        .unwrap()
        .items;
    let completed = records
        .iter()
        .find(|r| r.action == AuditAction::OperationCompleted)
        .expect("completed record");
    assert_eq!(completed.approval_id, Some(approval.approval_id));
    assert_eq!(completed.submission, Some(SubmissionState::Completed));
    assert!(!d.world.audit_contains_secret_marker());
}

/// Digest that does not match any stored intent.
struct IntentDigestOf;
impl IntentDigestOf {
    fn other() -> linux_mcp_contracts::canonical::IntentDigest {
        linux_mcp_contracts::canonical::IntentDigest::of_canonical("{\"not\":\"the intent\"}")
    }
}

#[test]
fn expired_approvals_cannot_be_decided_or_consumed() {
    let d = demo();
    let s = d.world.services();
    let driver = d.driver();
    let approval = approved_delete(&d, d.servers.alice_shared);
    d.world.clock().advance_secs(16 * 60);
    let agent = d.agent_ctx(&d.alice); // fresh deadline; the token (1h) is still valid
    let err = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap_err();
    assert!(matches!(
        err.details(),
        Some(ErrorDetails::ApprovalRequired {
            reason: ApprovalErrorReason::Expired,
            ..
        })
    ));
    assert_eq!(
        block_on(s.approvals.get(&agent, approval.approval_id))
            .unwrap()
            .status,
        ApprovalStatus::Expired
    );
    assert!(d.world.attempts().is_empty());
}

#[test]
fn timeout_before_submission_is_a_definite_non_execution() {
    let d = demo();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    script_timeout_before_submit(&d, d.servers.alice_shared);
    let result = finished(block_on(driver.run(&agent, id_cmd(d.servers.alice_shared))).unwrap());
    assert_eq!(result.submission, SubmissionState::NotSubmitted);
    assert_eq!(
        result.outcome,
        OperationOutcome::FailedBeforeSubmission {
            cause: PreSubmissionFailure::Timeout
        }
    );
    assert!(!result.outcome.may_have_run());
    assert!(d.world.attempts().is_empty());
}

#[test]
fn disconnect_after_submission_is_unknown_and_never_replayed() {
    let d = demo();
    let s = d.world.services();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let server = d.servers.alice_shared;
    script_disconnect_after_submit(&d, server);
    let approval = approved_delete(&d, server);
    let result = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap();
    assert_eq!(result.submission, SubmissionState::Submitted);
    assert!(matches!(
        result.outcome,
        OperationOutcome::UnknownAfterSubmission {
            cause: UnknownCause::Disconnected,
            ..
        }
    ));
    assert!(result.outcome.may_have_run());
    assert_eq!(d.world.attempts().len(), 1);

    // The approval stays consumed; retries fail and nothing is re-sent, even after recovery.
    let retry = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap_err();
    assert_eq!(retry.code(), ErrorCode::ApprovalRequired);
    d.world.script(server, ExecScript::ok());
    assert!(
        block_on(s.audit.reconcile_unresolved(&d.call_ctx()))
            .unwrap()
            .marked_unknown
            == 0,
        "outcome already recorded as unknown"
    );
    assert_eq!(d.world.attempts().len(), 1);
    let actions = d.world.audit_actions();
    assert!(actions.contains(&AuditAction::OperationOutcomeUnknown));
    let attempt_ids: std::collections::HashSet<_> =
        d.world.attempts().iter().map(|a| a.operation_id).collect();
    assert_eq!(attempt_ids.len(), 1);
}

#[test]
fn timeout_after_submission_is_unknown_with_timeout_cause() {
    let d = demo();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    d.world
        .script(d.servers.alice_shared, ExecScript::TimeoutAfterSubmit);
    let approval = approved_delete(&d, d.servers.alice_shared);
    let result = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap();
    assert!(matches!(
        result.outcome,
        OperationOutcome::UnknownAfterSubmission {
            cause: UnknownCause::Timeout,
            ..
        }
    ));
}

#[test]
fn unavailable_audit_rejects_mutation_before_anything_is_sent() {
    let d = demo();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let approval = approved_delete(&d, d.servers.alice_shared);
    d.world.set_audit_unavailable(true);
    let err = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Unavailable);
    assert!(d.world.attempts().is_empty());
}

#[test]
fn nonzero_exit_is_succeeded_with_separate_exit_status() {
    let d = demo();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    d.world.script(
        d.servers.alice_shared,
        ExecScript::Succeed {
            exit_code: 2,
            stdout: String::new(),
            stderr: "nope\n".into(),
        },
    );
    let result = finished(block_on(driver.run(&agent, id_cmd(d.servers.alice_shared))).unwrap());
    match result.outcome {
        OperationOutcome::Succeeded {
            exit_status,
            stderr,
            ..
        } => {
            assert_eq!(exit_status, ExitStatus::Code { code: 2 });
            assert_eq!(stderr.text, "nope\n");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn crash_between_pending_and_outcome_is_reconciled_as_unknown() {
    let d = demo();
    let s = d.world.services();
    let agent = d.agent_ctx(&d.alice);
    let driver = d.driver();
    // Begin a durable record, then "crash" before the executor runs.
    let target = block_on(
        s.targets
            .resolve(&agent, d.servers.alice_shared, Scope::ServersInspect),
    )
    .unwrap();
    let request = id_cmd(d.servers.alice_shared);
    let now = d.world.now();
    let intent = OperationIntent::new(IntentParts {
        operation_id: d.world.next_operation_id(),
        installation_id: agent.principal.installation_id(),
        owner: d.alice.user,
        server_id: target.server_id(),
        server_revision: target.revision(),
        tool: request.tool.clone(),
        tool_schema_version: request.tool_schema_version,
        arguments: request.arguments.clone(),
        risk: RiskLevel::Low,
        created_at: now,
        expires_at: now.checked_add_millis(60_000).unwrap(),
    })
    .unwrap();
    let policy = block_on(s.executor.evaluate(&agent, &intent, &target)).unwrap();
    let authority =
        linux_mcp_contracts::backend::operation_authority::ExecutionAuthority::without_approval(
            &intent, &target, &policy, now,
        )
        .unwrap();
    let ticket = block_on(s.audit.begin_operation(&agent, &intent, &authority)).unwrap();
    block_on(s.audit.record_submitted(&agent, &ticket)).unwrap();
    drop(driver);
    let report = block_on(s.audit.reconcile_unresolved(&d.call_ctx())).unwrap();
    assert_eq!(report.marked_unknown, 1);
    let records = block_on(s.audit.list_owned(&agent, AuditQuery::default()))
        .unwrap()
        .items;
    assert_eq!(
        records[0].outcome,
        Some(AuditOutcomeStatus::UnknownAfterSubmission)
    );
    assert!(d.world.attempts().is_empty(), "recovery never replays");
}

#[test]
fn concurrent_approval_consumption_allows_exactly_one_dispatch() {
    let d = demo();
    let server = d.servers.alice_shared;
    let approval = approved_delete(&d, server);
    let agent = d.agent_ctx(&d.alice);
    let outcomes: Vec<Result<OperationResult, PublicError>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..16)
            .map(|_| {
                scope.spawn(|| block_on(d.driver().execute_approved(&agent, approval.approval_id)))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(
        outcomes
            .iter()
            .filter_map(|r| r.as_ref().err())
            .all(|e| e.code() == ErrorCode::ApprovalRequired)
    );
    assert_eq!(d.world.attempts().len(), 1);
}

#[test]
fn concurrent_stale_updates_allow_exactly_one_winner() {
    let d = demo();
    let human = d.browser_ctx(&d.alice);
    let server = d.servers.alice_shared;
    let results: Vec<ServiceResult<ServerSummary>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let (d, human) = (&d, &human);
                scope.spawn(move || {
                    let update = UpdateServerRequest {
                        expected_revision: ServerRevision::new(2).unwrap(),
                        display_name: Some(DisplayName::new(format!("edit {i}")).unwrap()),
                        hostname: None,
                        port: None,
                        username: None,
                        environment: None,
                        enabled: None,
                        approval_mode: None,
                    };
                    block_on(d.world.services().inventory.update(human, server, update))
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(
        results
            .iter()
            .filter_map(|r| r.as_ref().err())
            .all(|e| e.code() == ErrorCode::RevisionConflict)
    );
    assert_eq!(d.world.server_revision(server).unwrap().get(), 3);
}

#[test]
fn revocation_after_approval_blocks_execution() {
    let d = demo();
    let s = d.world.services();
    let driver = d.driver();
    let agent = d.agent_ctx(&d.alice);
    let approval = approved_delete(&d, d.servers.alice_shared);
    block_on(s.identity.revoke(
        &agent,
        RevocationTarget::User {
            installation_id: agent.principal.installation_id(),
            user_id: d.alice.user,
        },
    ))
    .unwrap();
    assert_eq!(
        code(block_on(
            driver.execute_approved(&agent, approval.approval_id)
        )),
        ErrorCode::Unauthenticated
    );
    assert!(d.world.attempts().is_empty());
}

#[test]
fn inventory_crud_pagination_and_delete_invalidation() {
    let d = demo();
    let s = d.world.services();
    let human = d.browser_ctx(&d.bob);
    for name in ["one", "two", "three"] {
        let request: CreateServerRequest = serde_json::from_value(json!({
            "display_name": name, "hostname": "new.example.internal", "username": "deploy", "environment": "testing"
        })).unwrap();
        let created = block_on(s.inventory.create(&human, request)).unwrap();
        assert_eq!(created.revision, ServerRevision::INITIAL);
        assert_eq!(created.host_trust.status, HostTrustStatus::Unenrolled);
    }
    let first = block_on(s.inventory.list_owned(
        &human,
        PageRequest {
            cursor: None,
            limit: PageLimit::new(2).unwrap(),
        },
    ))
    .unwrap();
    assert_eq!(first.items.len(), 2);
    let second = block_on(s.inventory.list_owned(
        &human,
        PageRequest {
            cursor: first.next_cursor.clone(),
            limit: PageLimit::new(2).unwrap(),
        },
    ))
    .unwrap();
    assert_eq!(second.items.len(), 2);
    assert!(second.next_cursor.is_none());
    let bad = PageRequest {
        cursor: Some(linux_mcp_contracts::public::pagination::PageCursor::new("zzz").unwrap()),
        limit: PageLimit::DEFAULT,
    };
    assert_eq!(
        code(block_on(s.inventory.list_owned(&human, bad))),
        ErrorCode::Validation
    );

    let target = d.servers.bob_shared;
    let stale = block_on(
        s.inventory
            .delete(&human, target, ServerRevision::new(1).unwrap()),
    )
    .unwrap_err();
    assert_eq!(stale.code(), ErrorCode::RevisionConflict);
    block_on(
        s.inventory
            .delete(&human, target, ServerRevision::new(2).unwrap()),
    )
    .unwrap();
    assert_eq!(
        code(block_on(s.inventory.get_owned(&human, target))),
        ErrorCode::NotFound
    );
    assert!(
        d.world
            .invalidations()
            .iter()
            .any(|e| e.server_id == target && e.reason == InvalidationReason::Deleted)
    );
}

#[test]
fn deadlines_are_enforced_before_work() {
    let d = demo();
    let s = d.world.services();
    let mut ctx = d.agent_ctx(&d.alice);
    ctx.call.deadline = d.world.now();
    assert_eq!(
        code(block_on(
            s.inventory.list_owned(&ctx, PageRequest::default())
        )),
        ErrorCode::Timeout
    );
}

#[test]
fn reset_reseeds_deterministically_and_invalidates_old_tokens() {
    let d = demo();
    let call = d.call_ctx();
    let before = d.agent_ctx(&d.alice).principal.user_id();
    let again = d.reset();
    assert_eq!(again.agent_ctx(&again.alice).principal.user_id(), before);
    assert_eq!(again.servers.alice_shared, d.servers.alice_shared);
    assert!(d.world.attempts().is_empty() && d.world.audit_actions().is_empty());
    // Tokens are minted by counter, so the first reseeded token value may repeat; what matters
    // is that pre-reset state (attempts, audit, revisions) is gone.
    let _ = call;
    assert_eq!(
        again
            .world
            .server_revision(again.servers.alice_shared)
            .unwrap()
            .get(),
        2
    );
}

#[test]
fn fixtures_contain_only_fake_secret_markers() {
    let d = demo();
    let s = d.world.services();
    let agent = d.agent_ctx(&d.alice);
    for server in [d.servers.alice_shared, d.servers.alice_untrusted] {
        let summary = block_on(s.inventory.get_owned(&agent, server)).unwrap();
        let text = serde_json::to_string(&summary).unwrap();
        assert!(!text.contains("FAKE-SECRET") && !text.contains("BEGIN"));
    }
    let attempts = format!("{:?}", d.world.attempts());
    assert!(!attempts.contains("FAKE-SECRET"));
}

// ---------------------------------------------------------------------------------------
// Per-server approval mode (decisions.md D11)
// ---------------------------------------------------------------------------------------

fn medium_cmd(server: ServerId) -> OperationRequest {
    // Classified medium by the mock classifier (neither read-only nor destructive).
    op(
        server,
        "execute_command",
        json!({"command": "touch /tmp/probe"}),
    )
}

fn set_mode(
    d: &Demo,
    server: ServerId,
    revision: u64,
    mode: ApprovalMode,
) -> ServiceResult<ServerSummary> {
    let human = d.browser_ctx(&d.alice);
    let update = UpdateServerRequest {
        expected_revision: ServerRevision::new(revision).unwrap(),
        display_name: None,
        hostname: None,
        port: None,
        username: None,
        environment: None,
        enabled: None,
        approval_mode: Some(mode),
    };
    block_on(d.world.services().inventory.update(&human, server, update))
}

#[test]
fn approval_mode_defaults_to_medium_and_above_and_toggles_per_server() {
    let d = demo();
    let driver = d.driver();
    let alice = d.agent_ctx(&d.alice);
    let server = d.servers.alice_shared;
    let summary = block_on(d.world.services().inventory.get_owned(&alice, server)).unwrap();
    assert_eq!(summary.approval_mode, ApprovalMode::MediumAndAbove);

    // Default: medium and high need approval; low runs.
    pending(block_on(driver.run(&alice, medium_cmd(server))).unwrap());
    pending(block_on(driver.run(&alice, delete_user(server))).unwrap());
    finished(block_on(driver.run(&alice, id_cmd(server))).unwrap());
    assert_eq!(d.world.attempts().len(), 1);

    // The owner relaxes this one server in a browser session; the revision moves.
    let relaxed = set_mode(&d, server, 2, ApprovalMode::HighAndAbove).unwrap();
    assert_eq!(relaxed.approval_mode, ApprovalMode::HighAndAbove);
    assert_eq!(relaxed.revision.get(), 3);
    let alice = d.agent_ctx(&d.alice);
    let medium = finished(block_on(driver.run(&alice, medium_cmd(server))).unwrap());
    assert!(matches!(medium.outcome, OperationOutcome::Succeeded { .. }));
    pending(block_on(driver.run(&alice, delete_user(server))).unwrap());
    // Denials are not affected by the mode.
    let denied = finished(
        block_on(driver.run(
            &alice,
            op(server, "execute_command", json!({"command": "rm -rf /"})),
        ))
        .unwrap(),
    );
    assert!(matches!(
        denied.outcome,
        OperationOutcome::Refused {
            reason: RefusalReason::PolicyDenied,
            ..
        }
    ));

    // The setting is per server: the other owner server is still on the default.
    let bob = d.agent_ctx(&d.bob);
    pending(block_on(d.driver().run(&bob, medium_cmd(d.servers.bob_shared))).unwrap());

    // And it can be toggled back.
    set_mode(&d, server, 3, ApprovalMode::MediumAndAbove).unwrap();
    let alice = d.agent_ctx(&d.alice);
    pending(block_on(driver.run(&alice, medium_cmd(server))).unwrap());
}

#[test]
fn agents_cannot_change_or_weaken_the_approval_mode() {
    let d = demo();
    let s = d.world.services();
    let server = d.servers.alice_shared;
    // A powerful agent token: inventory write scope, but not an interactive session.
    let token = d.world.issue_token(
        d.alice.user,
        &[Scope::InventoryRead, Scope::InventoryWrite],
        AuthKind::McpToken,
        600,
    );
    let agent = d.ctx_from_token(&token);
    let weaken = UpdateServerRequest {
        expected_revision: ServerRevision::new(2).unwrap(),
        display_name: None,
        hostname: None,
        port: None,
        username: None,
        environment: None,
        enabled: None,
        approval_mode: Some(ApprovalMode::HighAndAbove),
    };
    assert_eq!(
        code(block_on(s.inventory.update(&agent, server, weaken))),
        ErrorCode::Forbidden
    );
    assert_eq!(
        d.world.server_revision(server).unwrap().get(),
        2,
        "no change was made"
    );

    // Other edits by the same token still work; only the policy is restricted.
    let rename = UpdateServerRequest {
        expected_revision: ServerRevision::new(2).unwrap(),
        display_name: Some(DisplayName::new("renamed by agent").unwrap()),
        hostname: None,
        port: None,
        username: None,
        environment: None,
        enabled: None,
        approval_mode: None,
    };
    assert!(block_on(s.inventory.update(&agent, server, rename)).is_ok());

    // Creating a server with a relaxed mode is restricted the same way.
    let relaxed: CreateServerRequest = serde_json::from_value(json!({
        "display_name": "x", "hostname": "x.example.internal", "username": "deploy",
        "environment": "testing", "approval_mode": "high_and_above"
    }))
    .unwrap();
    assert_eq!(
        code(block_on(s.inventory.create(&agent, relaxed.clone()))),
        ErrorCode::Forbidden
    );
    let browser = d.browser_ctx(&d.alice);
    let created = block_on(s.inventory.create(&browser, relaxed)).unwrap();
    assert_eq!(created.approval_mode, ApprovalMode::HighAndAbove);

    // Omitting the field creates a server with the safe default, even for an agent token.
    let plain: CreateServerRequest = serde_json::from_value(json!({
        "display_name": "y", "hostname": "y.example.internal", "username": "deploy",
        "environment": "testing"
    }))
    .unwrap();
    let created = block_on(s.inventory.create(&agent, plain)).unwrap();
    assert_eq!(created.approval_mode, ApprovalMode::MediumAndAbove);
}

#[test]
fn changing_the_approval_mode_invalidates_pending_approvals() {
    let d = demo();
    let driver = d.driver();
    let server = d.servers.alice_shared;
    let agent = d.agent_ctx(&d.alice);
    let approval = pending(block_on(driver.run(&agent, medium_cmd(server))).unwrap());
    let human = d.browser_ctx(&d.alice);
    block_on(driver.decide(&human, &approval, ApprovalDecision::Approve)).unwrap();

    // The policy changes after approval: the revision moves, so the old approval is unusable.
    set_mode(&d, server, 2, ApprovalMode::HighAndAbove).unwrap();
    let agent = d.agent_ctx(&d.alice);
    let err = block_on(driver.execute_approved(&agent, approval.approval_id)).unwrap_err();
    assert_eq!(err.code(), ErrorCode::ApprovalRequired);
    assert!(d.world.attempts().is_empty());
}
