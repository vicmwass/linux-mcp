//! Step 00-05: canonicalization, intents, outcomes and (backend) execution authority.

#![cfg(feature = "public")]

use linux_mcp_contracts::canonical::{
    CANONICALIZATION_VERSION, CanonicalError, IntentDigest, MAX_DEPTH, canonicalize,
};
use linux_mcp_contracts::public::ids::{
    InstallationId, OperationId, ServerId, ServerRevision, Timestamp, UserId,
};
use linux_mcp_contracts::public::operations::{
    BoundedOutput, CanonicalArguments, IntentParts, OperationIntent, OperationOutcome,
    OperationRequest, RiskLevel, SubmissionState, ToolId, ToolSchemaVersion, TransportFailure,
    UnknownCause,
};
use serde_json::{Value, json};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/operations");

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{FIXTURES}/{name}")).unwrap()
}

fn intent_fixture() -> OperationIntent {
    serde_json::from_str(&fixture("intent.json")).unwrap()
}

fn parts(intent: &OperationIntent) -> IntentParts {
    IntentParts {
        operation_id: intent.operation_id(),
        installation_id: intent.installation_id(),
        owner: intent.owner(),
        server_id: intent.server_id(),
        server_revision: intent.server_revision(),
        tool: intent.tool().clone(),
        tool_schema_version: intent.tool_schema_version(),
        arguments: intent.arguments().clone(),
        risk: intent.risk(),
        created_at: intent.created_at(),
        expires_at: intent.expires_at(),
    }
}

#[test]
fn canonical_form_is_order_and_whitespace_independent() {
    let a: Value =
        serde_json::from_str(r#"{ "b": 1, "a": [true, {"z": "x", "y": null}], "c": "é" }"#)
            .unwrap();
    let b: Value = serde_json::from_str(r#"{"c":"é","a":[true,{"z":"x"}],"b":1}"#).unwrap();
    assert_eq!(canonicalize(&a).unwrap(), canonicalize(&b).unwrap());
    assert_eq!(
        canonicalize(&a).unwrap(),
        r#"{"a":[true,{"z":"x"}],"b":1,"c":"é"}"#
    );
    // Keys sort by UTF-8 bytes: uppercase before lowercase, ASCII before non-ASCII.
    assert_eq!(
        canonicalize(&json!({"b": 0, "B": 0, "ä": 0})).unwrap(),
        r#"{"B":0,"b":0,"ä":0}"#
    );
    // Strings are not Unicode-normalized: NFC and NFD forms differ.
    assert_ne!(
        canonicalize(&json!({"u": "\u{e9}"})).unwrap(),
        canonicalize(&json!({"u": "e\u{301}"})).unwrap()
    );
    // Array order is significant.
    assert_ne!(
        canonicalize(&json!([1, 2])).unwrap(),
        canonicalize(&json!([2, 1])).unwrap()
    );
}

#[test]
fn canonical_form_rejects_floats_depth_size_and_secrets() {
    assert!(matches!(
        canonicalize(&json!({"x": 1.5})),
        Err(CanonicalError::FloatNotAllowed(_))
    ));
    assert!(canonicalize(&json!({"x": -9_223_372_036_854_775_808i64, "y": u64::MAX})).is_ok());

    let mut deep = json!(1);
    for _ in 0..=MAX_DEPTH {
        deep = json!([deep]);
    }
    assert!(matches!(
        canonicalize(&deep),
        Err(CanonicalError::TooDeep(_))
    ));

    let big = json!({"x": "a".repeat(70 * 1024)});
    assert_eq!(canonicalize(&big), Err(CanonicalError::TooLarge));

    for key in [
        "password",
        "sudo_password",
        "Private-Key",
        "api_token",
        "client_secret",
        "passphrase",
    ] {
        let value = json!({"nested": [{ key: "FAKE" }]});
        assert!(
            matches!(canonicalize(&value), Err(CanonicalError::SensitiveField(_))),
            "{key} must be rejected"
        );
    }
}

#[test]
fn canonical_arguments_must_be_objects_and_validate_on_deserialize() {
    assert_eq!(
        CanonicalArguments::new(json!([1])),
        Err(CanonicalError::NotAnObject)
    );
    let args = CanonicalArguments::new(json!({"b": null, "a": 1})).unwrap();
    assert_eq!(args.canonical(), r#"{"a":1}"#);
    assert_eq!(serde_json::to_value(&args).unwrap(), json!({"a": 1}));
    assert!(serde_json::from_value::<CanonicalArguments>(json!({"password": "x"})).is_err());

    let request: OperationRequest = serde_json::from_value(json!({
        "server_id": "5e000000-0000-4000-8000-00000000a001",
        "tool": "execute_command",
        "tool_schema_version": 1,
        "arguments": {"command": "id", "confirm": false}
    }))
    .unwrap();
    assert_eq!(request.tool.as_str(), "execute_command");
    assert!(
        serde_json::from_value::<OperationRequest>(json!({
            "server_id": "5e000000-0000-4000-8000-00000000a001",
            "tool": "execute_command", "tool_schema_version": 1, "arguments": {},
            "owner": "a11ce000-0000-4000-8000-000000000001"
        }))
        .is_err()
    );
    assert!(ToolId::new("Execute").is_err());
    assert!(ToolSchemaVersion::new(0).is_err());
}

#[test]
fn intent_fixture_round_trips_and_digest_is_pinned() {
    let text = fixture("intent.json");
    let intent = intent_fixture();
    let expected: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(serde_json::to_value(&intent).unwrap(), expected);
    assert_eq!(intent.canonicalization(), CANONICALIZATION_VERSION);
    let pinned = fixture("intent.digest");
    assert_eq!(
        intent.digest().as_str(),
        pinned.trim(),
        "canonicalization drift"
    );
    assert!(IntentDigest::parse(pinned.trim()).is_ok());
}

#[test]
fn digest_binds_every_field_except_request_identity() {
    let base = intent_fixture();
    let digest = base.digest();

    // operation_id and created_at identify the request, not what is approved.
    let mut same = parts(&base);
    same.operation_id = OperationId::parse("0e000000-0000-4000-8000-000000000099").unwrap();
    same.created_at = Timestamp::parse("2026-10-11T08:01:00Z").unwrap();
    assert_eq!(OperationIntent::new(same).unwrap().digest(), digest);

    type Mutation = fn(&mut IntentParts);
    let mutations: Vec<(&str, Mutation)> = vec![
        ("installation", |p| {
            p.installation_id =
                InstallationId::parse("aaaaaaaa-0000-4000-8000-000000000002").unwrap()
        }),
        ("owner", |p| {
            p.owner = UserId::parse("b0b00000-0000-4000-8000-000000000002").unwrap()
        }),
        ("server", |p| {
            p.server_id = ServerId::parse("5e000000-0000-4000-8000-00000000b001").unwrap()
        }),
        ("revision", |p| {
            p.server_revision = ServerRevision::new(3).unwrap()
        }),
        ("tool", |p| p.tool = ToolId::new("disable_user").unwrap()),
        ("schema", |p| {
            p.tool_schema_version = ToolSchemaVersion::new(2).unwrap()
        }),
        ("arguments", |p| {
            p.arguments = CanonicalArguments::new(
                json!({"username": "olduser", "remove_home": false, "confirm": true}),
            )
            .unwrap()
        }),
        ("risk", |p| p.risk = RiskLevel::Medium),
        ("expiry", |p| {
            p.expires_at = Timestamp::parse("2026-10-11T08:20:00Z").unwrap()
        }),
    ];
    for (name, mutate) in mutations {
        let mut changed = parts(&base);
        mutate(&mut changed);
        assert_ne!(
            OperationIntent::new(changed).unwrap().digest(),
            digest,
            "{name} must change the digest"
        );
    }
}

#[test]
fn intent_lifetime_is_bounded_and_versions_are_checked() {
    let base = intent_fixture();
    let mut expired = parts(&base);
    expired.expires_at = expired.created_at;
    assert!(OperationIntent::new(expired).is_err());
    let mut too_long = parts(&base);
    too_long.expires_at = too_long
        .created_at
        .checked_add_millis(25 * 3_600_000)
        .unwrap();
    assert!(OperationIntent::new(too_long).is_err());
    assert!(base.is_expired_at(base.expires_at()));
    assert!(!base.is_expired_at(base.created_at()));

    let mut value: Value = serde_json::from_str(&fixture("intent.json")).unwrap();
    value["canonicalization"] = json!("lmcp-canon-v0");
    let old: OperationIntent = serde_json::from_value(value).unwrap();
    assert!(old.check_version().is_err());
    assert_ne!(old.digest(), base.digest());
}

#[test]
fn every_outcome_variant_round_trips() {
    let outcomes: Vec<Value> = serde_json::from_str(&fixture("outcomes.json")).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for value in outcomes {
        let outcome: OperationOutcome = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&outcome).unwrap(), value);
        seen.insert(value["status"].as_str().unwrap().to_owned());
    }
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        [
            "failed_before_submission",
            "refused",
            "succeeded",
            "unknown_after_submission"
        ]
    );
    // Unknown status values are rejected rather than guessed.
    assert!(serde_json::from_value::<OperationOutcome>(json!({"status": "failed"})).is_err());
    // An unknown outcome cannot carry an exit status.
    assert!(
        serde_json::from_value::<OperationOutcome>(json!({
            "status": "unknown_after_submission", "cause": "disconnected",
            "partial_stdout": null, "partial_stderr": null,
            "exit_status": {"kind": "code", "code": 1}
        }))
        .is_err()
    );
}

#[test]
fn disconnect_after_submission_is_never_a_definite_failure() {
    for failure in [
        TransportFailure::Timeout,
        TransportFailure::Cancelled,
        TransportFailure::Disconnected,
        TransportFailure::ConnectionFailed,
    ] {
        let before = OperationOutcome::transport_failure(SubmissionState::NotSubmitted, failure);
        assert!(matches!(
            before,
            OperationOutcome::FailedBeforeSubmission { .. }
        ));
        assert!(!before.may_have_run());
        for state in [
            SubmissionState::SubmissionPending,
            SubmissionState::Submitted,
            SubmissionState::Completed,
        ] {
            let after = OperationOutcome::transport_failure(state, failure);
            assert!(
                matches!(after, OperationOutcome::UnknownAfterSubmission { .. }),
                "{state:?} {failure:?}"
            );
            assert!(after.may_have_run());
        }
    }
    assert!(matches!(
        OperationOutcome::transport_failure(
            SubmissionState::Submitted,
            TransportFailure::Disconnected
        ),
        OperationOutcome::UnknownAfterSubmission {
            cause: UnknownCause::Disconnected,
            ..
        }
    ));
}

#[test]
fn bounded_output_truncates_on_char_boundary() {
    let out = BoundedOutput::from_bytes("aé€😀".as_bytes(), 4);
    assert_eq!(out.text, "aé");
    assert!(out.truncated);
    assert_eq!(out.original_bytes, 10);
    assert!(!out.lossy);
    let lossy = BoundedOutput::from_bytes(&[b'o', b'k', 0xff], 100);
    assert!(lossy.lossy);
    assert!(!lossy.truncated);
    assert_eq!(lossy.text, "ok\u{fffd}");
    let detail = "x".repeat(2_000);
    match OperationOutcome::refused(
        linux_mcp_contracts::public::operations::RefusalReason::PolicyDenied,
        Some(&detail),
    ) {
        OperationOutcome::Refused {
            detail: Some(d), ..
        } => assert_eq!(d.len(), 512),
        other => panic!("{other:?}"),
    }
}

#[cfg(feature = "backend")]
mod authority {
    use super::*;
    use std::collections::BTreeSet;

    use linux_mcp_contracts::backend::identity::{
        AuthKind, Principal, Scope, SessionRef, VerifiedIdentity,
    };
    use linux_mcp_contracts::backend::operation_authority::{
        ApprovalEvidence, AuthorityError, ExecutionAuthority, PolicyEvaluation,
    };
    use linux_mcp_contracts::backend::target::{
        AuthorizedTarget, HostTrustRef, NetworkTarget, ResolvedTarget,
    };
    use linux_mcp_contracts::public::ids::{ApprovalId, HostTrustId};
    use linux_mcp_contracts::public::inventory::{
        ApprovalMode, HostTrustStatus, Hostname, SshPort, SshUsername,
    };
    use linux_mcp_contracts::public::operations::{PolicyVerdict, RefusalReason};

    const APPROVAL: &str = "a9900000-0000-4000-8000-000000000001";

    fn now() -> Timestamp {
        Timestamp::parse("2026-10-11T08:05:00Z").unwrap()
    }

    fn principal(user: UserId, kind: AuthKind, scopes: &[Scope]) -> Principal {
        Principal::from_verified(VerifiedIdentity {
            user_id: user,
            installation_id: InstallationId::parse("aaaaaaaa-0000-4000-8000-000000000001").unwrap(),
            scopes: scopes.iter().copied().collect::<BTreeSet<_>>(),
            session: SessionRef::new("s-1").unwrap(),
            auth_kind: kind,
            authenticated_at: Timestamp::parse("2026-10-11T07:00:00Z").unwrap(),
            expires_at: Timestamp::parse("2026-10-11T09:00:00Z").unwrap(),
        })
        .unwrap()
    }

    fn target(intent: &OperationIntent, revision: u64) -> AuthorizedTarget {
        let hostname = Hostname::new("shared.example.internal").unwrap();
        let resolved = ResolvedTarget {
            owner: intent.owner(),
            installation_id: intent.installation_id(),
            server_id: intent.server_id(),
            revision: ServerRevision::new(revision).unwrap(),
            enabled: true,
            approval_mode: ApprovalMode::default(),
            network: NetworkTarget {
                hostname: hostname.clone(),
                port: SshPort::DEFAULT,
                username: SshUsername::new("deploy").unwrap(),
            },
            host_trust: Some(HostTrustRef {
                id: HostTrustId::parse("70000000-0000-4000-8000-000000000001").unwrap(),
                status: HostTrustStatus::Verified,
                fingerprint: "SHA256:fake".into(),
                hostname,
                port: SshPort::DEFAULT,
            }),
            credentials: vec![],
        };
        let owner = principal(intent.owner(), AuthKind::McpToken, &[Scope::ServersMutate]);
        AuthorizedTarget::authorize(&owner, resolved, Scope::ServersMutate, now()).unwrap()
    }

    fn policy(intent: &OperationIntent, verdict: PolicyVerdict) -> PolicyEvaluation {
        PolicyEvaluation {
            intent_digest: intent.digest(),
            server_revision: intent.server_revision(),
            verdict,
            policy_version: "policy-v1".into(),
            evaluated_at: now(),
        }
    }

    fn approve(intent: &OperationIntent) -> ApprovalEvidence {
        let human = principal(
            intent.owner(),
            AuthKind::BrowserSession,
            &[Scope::ApprovalsDecide],
        );
        ApprovalEvidence::record(ApprovalId::parse(APPROVAL).unwrap(), &human, intent, now())
            .unwrap()
    }

    #[test]
    fn exact_binding_produces_authority() {
        let intent = intent_fixture();
        let target = target(&intent, 2);
        let policy = policy(&intent, PolicyVerdict::RequiresApproval);
        let authority = ExecutionAuthority::from_consumed_approval(
            approve(&intent),
            &intent,
            &target,
            &policy,
            now(),
        )
        .unwrap();
        assert_eq!(
            authority.approval_id(),
            Some(ApprovalId::parse(APPROVAL).unwrap())
        );
        assert_eq!(authority.intent_digest(), &intent.digest());
        assert!(authority.recheck(&intent, &target, &policy, now()).is_ok());
    }

    #[test]
    fn agents_cannot_approve_themselves() {
        let intent = intent_fixture();
        let id = ApprovalId::parse(APPROVAL).unwrap();
        for (kind, scopes) in [
            (
                AuthKind::McpToken,
                vec![Scope::ApprovalsDecide, Scope::ServersMutate],
            ),
            (AuthKind::DelegatedToken, vec![Scope::ApprovalsDecide]),
            (AuthKind::LegacyLocal, vec![Scope::ApprovalsDecide]),
            (
                AuthKind::BrowserSession,
                vec![Scope::ServersMutate, Scope::ApprovalsRequest],
            ),
        ] {
            let p = principal(intent.owner(), kind, &scopes);
            assert_eq!(
                ApprovalEvidence::record(id, &p, &intent, now()).unwrap_err(),
                AuthorityError::ApproverNotAllowed,
                "{kind:?}"
            );
        }
        let bob = principal(
            UserId::parse("b0b00000-0000-4000-8000-000000000002").unwrap(),
            AuthKind::BrowserSession,
            &[Scope::ApprovalsDecide],
        );
        assert!(ApprovalEvidence::record(id, &bob, &intent, now()).is_err());
    }

    #[test]
    fn altered_arguments_invalidate_approval() {
        let intent = intent_fixture();
        let evidence = approve(&intent);
        let mut altered = parts(&intent);
        altered.arguments = CanonicalArguments::new(
            json!({"username": "root", "remove_home": true, "confirm": true}),
        )
        .unwrap();
        let altered = OperationIntent::new(altered).unwrap();
        let target = target(&altered, 2);
        let policy = policy(&altered, PolicyVerdict::RequiresApproval);
        assert_eq!(
            ExecutionAuthority::from_consumed_approval(evidence, &altered, &target, &policy, now())
                .unwrap_err(),
            AuthorityError::BindingMismatch
        );
    }

    #[test]
    fn changed_revision_and_expiry_invalidate_approval() {
        let intent = intent_fixture();
        let moved = target(&intent, 3);
        let policy = policy(&intent, PolicyVerdict::RequiresApproval);
        assert!(matches!(
            ExecutionAuthority::from_consumed_approval(
                approve(&intent),
                &intent,
                &moved,
                &policy,
                now()
            ),
            Err(AuthorityError::RevisionChanged { .. })
        ));

        let target = target(&intent, 2);
        let late = intent.expires_at();
        assert_eq!(
            ExecutionAuthority::from_consumed_approval(
                approve(&intent),
                &intent,
                &target,
                &policy,
                late
            )
            .unwrap_err(),
            AuthorityError::Expired
        );
    }

    #[test]
    fn approval_never_overrides_policy_denial() {
        let intent = intent_fixture();
        let target = target(&intent, 2);
        let denied = policy(
            &intent,
            PolicyVerdict::Denied {
                reason: RefusalReason::PolicyDenied,
            },
        );
        let err = ExecutionAuthority::from_consumed_approval(
            approve(&intent),
            &intent,
            &target,
            &denied,
            now(),
        )
        .unwrap_err();
        assert_eq!(
            err,
            AuthorityError::PolicyDenied(RefusalReason::PolicyDenied)
        );

        // Authority minted under approval is rejected at submit if policy now denies.
        let ok = policy(&intent, PolicyVerdict::RequiresApproval);
        let authority = ExecutionAuthority::from_consumed_approval(
            approve(&intent),
            &intent,
            &target,
            &ok,
            now(),
        )
        .unwrap();
        assert!(authority.recheck(&intent, &target, &denied, now()).is_err());
    }

    #[test]
    fn approval_required_cannot_be_skipped() {
        let intent = intent_fixture();
        let target = target(&intent, 2);
        let requires = policy(&intent, PolicyVerdict::RequiresApproval);
        assert_eq!(
            ExecutionAuthority::without_approval(&intent, &target, &requires, now()).unwrap_err(),
            AuthorityError::ApprovalRequired
        );
        assert_eq!(
            AuthorityError::ApprovalRequired.refusal_reason(),
            RefusalReason::ApprovalRequired
        );
        let allowed = policy(&intent, PolicyVerdict::Allowed);
        let authority =
            ExecutionAuthority::without_approval(&intent, &target, &allowed, now()).unwrap();
        assert_eq!(authority.approval_id(), None);
    }

    #[test]
    fn policy_for_another_intent_is_rejected() {
        let intent = intent_fixture();
        let target = target(&intent, 2);
        let mut other = parts(&intent);
        other.arguments =
            CanonicalArguments::new(json!({"username": "x", "remove_home": true, "confirm": true}))
                .unwrap();
        let other = OperationIntent::new(other).unwrap();
        let wrong_policy = policy(&other, PolicyVerdict::Allowed);
        assert_eq!(
            ExecutionAuthority::without_approval(&intent, &target, &wrong_policy, now())
                .unwrap_err(),
            AuthorityError::BindingMismatch
        );
    }
}
