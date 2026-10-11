//! Step 00-03: public IDs, inventory DTOs, pagination and errors.

#![cfg(feature = "public")]

use linux_mcp_contracts::public::errors::{
    ApprovalErrorReason, ErrorCode, ErrorDetails, PublicError, ValidationError, ViolationRule,
};
use linux_mcp_contracts::public::ids::{
    ApprovalId, RequestId, ServerId, ServerRevision, Timestamp, UserId,
};
use linux_mcp_contracts::public::inventory::{
    CreateServerRequest, CredentialKind, CredentialState, CredentialStatus, DisplayName,
    Environment, HostTrustStatus, HostTrustSummary, Hostname, ServerSummary, SshPort, SshUsername,
    UpdateServerRequest,
};
use linux_mcp_contracts::public::pagination::{Page, PageCursor, PageLimit, PageRequest};
use serde_json::{Value, json};

const SERVER: &str = "3f2b8c1e-5d4a-4b6e-9f10-2a3b4c5d6e7f";
const REQUEST: &str = "0b1c2d3e-4f50-4617-8899-aabbccddeeff";

fn round_trip<T>(value: Value) -> T
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let parsed: T = serde_json::from_value(value.clone()).expect("fixture parses");
    let again = serde_json::to_value(&parsed).expect("serializes");
    assert_eq!(again, value, "round trip must be exact");
    parsed
}

fn summary_fixture() -> Value {
    json!({
        "id": SERVER,
        "display_name": "Ubuntu build box",
        "hostname": "build-01.example.internal",
        "port": 22,
        "username": "deploy",
        "environment": "development",
        "revision": 3,
        "enabled": true,
        "approval_mode": "medium_and_above",
        "host_trust": {
            "status": "verified",
            "fingerprint": "SHA256:AAAAexamplefingerprintAAAA",
            "verified_at": "2026-10-01T12:00:00.000Z"
        },
        "credentials": [
            {"kind": "ssh_private_key", "state": "provisioned",
             "credential_id": "11111111-2222-4333-8444-555555555555",
             "updated_at": "2026-10-01T12:00:00.000Z"},
            {"kind": "sudo_password", "state": "not_required",
             "credential_id": null, "updated_at": null}
        ],
        "created_at": "2026-09-30T08:00:00.000Z",
        "updated_at": "2026-10-01T12:00:00.000Z"
    })
}

#[test]
fn ids_round_trip_canonical_form() {
    let id = ServerId::parse(SERVER).unwrap();
    assert_eq!(id.to_string(), SERVER);
    assert_eq!(serde_json::to_value(id).unwrap(), json!(SERVER));
    let back: ServerId = serde_json::from_value(json!(SERVER)).unwrap();
    assert_eq!(back, id);
    assert_eq!(format!("{id:?}"), format!("ServerId({SERVER})"));
}

#[test]
fn ids_reject_non_canonical_forms() {
    for bad in [
        "",
        "not-a-uuid",
        "3F2B8C1E-5D4A-4B6E-9F10-2A3B4C5D6E7F",
        "{3f2b8c1e-5d4a-4b6e-9f10-2a3b4c5d6e7f}",
        "urn:uuid:3f2b8c1e-5d4a-4b6e-9f10-2a3b4c5d6e7f",
        "3f2b8c1e5d4a4b6e9f102a3b4c5d6e7f",
        "00000000-0000-0000-0000-000000000000",
        " 3f2b8c1e-5d4a-4b6e-9f10-2a3b4c5d6e7f",
    ] {
        assert!(ServerId::parse(bad).is_err(), "{bad:?} must be rejected");
        assert!(serde_json::from_value::<UserId>(json!(bad)).is_err());
    }
    assert!(serde_json::from_value::<ServerId>(json!(42)).is_err());
}

#[test]
fn revision_is_bounded_and_checked() {
    assert_eq!(ServerRevision::INITIAL.get(), 1);
    assert_eq!(ServerRevision::INITIAL.next().unwrap().get(), 2);
    assert!(ServerRevision::new(0).is_err());
    let max = ServerRevision::new(ServerRevision::MAX_VALUE).unwrap();
    assert!(max.next().is_err());
    assert!(serde_json::from_value::<ServerRevision>(json!(0)).is_err());
    assert!(serde_json::from_value::<ServerRevision>(json!(-1)).is_err());
    assert!(serde_json::from_value::<ServerRevision>(json!(9007199254740992u64)).is_err());
    assert!(serde_json::from_value::<ServerRevision>(json!("3")).is_err());
}

#[test]
fn timestamps_are_utc_millisecond_rfc3339() {
    let ts = Timestamp::parse("2026-10-11T07:38:59.810256Z").unwrap();
    assert_eq!(ts.to_string(), "2026-10-11T07:38:59.810Z");
    let ts: Timestamp = serde_json::from_value(json!("2026-10-11T07:38:59Z")).unwrap();
    assert_eq!(
        serde_json::to_value(ts).unwrap(),
        json!("2026-10-11T07:38:59.000Z")
    );
    assert!(Timestamp::parse("2026-10-11T07:38:59+03:00").is_err());
    assert!(Timestamp::parse("2026-10-11 07:38:59Z").is_err());
    assert!(Timestamp::parse("yesterday").is_err());
    assert_eq!(
        ts.checked_add_millis(1_500).unwrap().to_string(),
        "2026-10-11T07:39:00.500Z"
    );
}

#[test]
fn server_summary_round_trips_without_secrets() {
    let summary: ServerSummary = round_trip(summary_fixture());
    assert!(summary.host_trust.status.permits_connection());
    let text = serde_json::to_string(&summary).unwrap();
    for forbidden in [
        "\"private_key\":",
        "\"password\":",
        "\"secret\":",
        "\"owner",
        "/keys/",
        "-----BEGIN",
    ] {
        assert!(!text.contains(forbidden), "summary leaked {forbidden}");
    }
}

#[test]
fn server_summary_rejects_extra_fields() {
    let mut fixture = summary_fixture();
    fixture["private_key"] = json!("./keys/id_ed25519");
    assert!(serde_json::from_value::<ServerSummary>(fixture).is_err());
}

#[test]
fn create_request_defaults_and_rejects_owner() {
    let request: CreateServerRequest = serde_json::from_value(json!({
        "display_name": "  Mint laptop ",
        "hostname": "MINT.Example.org",
        "username": "victor",
        "environment": "production"
    }))
    .unwrap();
    assert_eq!(request.port, SshPort::DEFAULT);
    assert!(request.enabled);
    assert_eq!(request.display_name.as_str(), "Mint laptop");
    assert_eq!(request.hostname.as_str(), "mint.example.org");

    for owner_field in ["owner_user_id", "owner", "user_id"] {
        let mut value = json!({
            "display_name": "x", "hostname": "h.example", "username": "u",
            "environment": "development"
        });
        value[owner_field] = json!(UserId::parse(SERVER).unwrap());
        assert!(
            serde_json::from_value::<CreateServerRequest>(value).is_err(),
            "{owner_field} must be rejected"
        );
    }
}

#[test]
fn field_validation() {
    assert!(DisplayName::new("").is_err());
    assert!(DisplayName::new("   ").is_err());
    assert!(DisplayName::new("a".repeat(101)).is_err());
    assert!(DisplayName::new("tab\there").is_err());
    assert!(DisplayName::new("Ünïcode ✓").is_ok());

    for ok in ["10.0.0.10", "::1", "db-01.example.com", "localhost", "a"] {
        assert!(Hostname::new(ok).is_ok(), "{ok}");
    }
    assert_eq!(
        Hostname::new("2001:DB8:0:0:0:0:0:1").unwrap().as_str(),
        "2001:db8::1"
    );
    for bad in [
        "",
        "http://host",
        "host:22",
        "host/path",
        "-lead.example",
        "trail-.example",
        "a..b",
        "999.1.1.1",
        "[::1]",
        "host name",
        "host;rm",
        &"a".repeat(64),
    ] {
        assert!(Hostname::new(bad).is_err(), "{bad:?}");
    }

    for ok in ["root", "_svc", "deploy-1", "a"] {
        assert!(SshUsername::new(ok).is_ok(), "{ok}");
    }
    for bad in ["", "Root", "1abc", "has space", "x;y", &"a".repeat(33)] {
        assert!(SshUsername::new(bad).is_err(), "{bad:?}");
    }

    assert!(SshPort::new(0).is_err());
    assert!(serde_json::from_value::<SshPort>(json!(70000)).is_err());
    assert!(serde_json::from_value::<Environment>(json!("prod")).is_err());
}

#[test]
fn update_requires_revision_and_a_change() {
    let update: UpdateServerRequest = round_trip(json!({
        "expected_revision": 3,
        "hostname": "new.example.org"
    }));
    assert!(update.validate().is_ok());
    assert!(update.changes_target());

    let empty: UpdateServerRequest =
        serde_json::from_value(json!({"expected_revision": 3})).unwrap();
    let error = empty.validate().unwrap_err();
    assert_eq!(error.violations[0].rule, ViolationRule::NoChanges);

    assert!(
        serde_json::from_value::<UpdateServerRequest>(json!({"hostname": "x.example"})).is_err()
    );
    assert!(
        serde_json::from_value::<UpdateServerRequest>(
            json!({"expected_revision": 3, "enabled": false, "owner_user_id": SERVER})
        )
        .is_err()
    );
}

#[test]
fn pagination_contracts() {
    let page: PageRequest = serde_json::from_value(json!({})).unwrap();
    assert_eq!(page.limit, PageLimit::DEFAULT);
    assert!(page.cursor.is_none());
    let page: PageRequest = round_trip(json!({"cursor": "eyJvIjoxMH0", "limit": 200}));
    assert_eq!(page.limit.get(), 200);
    assert!(serde_json::from_value::<PageRequest>(json!({"limit": 0})).is_err());
    assert!(serde_json::from_value::<PageRequest>(json!({"limit": 201})).is_err());
    assert!(PageCursor::new("a+b/c=").is_err());
    assert!(PageCursor::new("").is_err());
    assert!(PageCursor::new("a".repeat(513)).is_err());

    let listed: Page<ServerSummary> =
        round_trip(json!({"items": [summary_fixture()], "next_cursor": null}));
    assert_eq!(listed.items.len(), 1);
}

#[test]
fn errors_are_sanitized_and_mapped() {
    let request_id = RequestId::parse(REQUEST).unwrap();
    for code in ErrorCode::ALL {
        let status = code.http_status();
        assert!((400..600).contains(&status));
        let wire = serde_json::to_value(code).unwrap();
        assert_eq!(wire, json!(code.as_str()));
    }

    let error = PublicError::not_found(request_id);
    let value = round_trip::<PublicError>(json!({
        "code": "not_found",
        "message": "The requested resource was not found.",
        "request_id": REQUEST,
        "details": null
    }));
    assert_eq!(value, error);
    assert_eq!(error.http_status(), 404);

    let conflict = PublicError::revision_conflict(request_id, ServerRevision::new(7).unwrap());
    assert_eq!(conflict.http_status(), 409);
    assert_eq!(
        serde_json::to_value(&conflict).unwrap()["details"],
        json!({"kind": "revision_conflict", "current_revision": 7})
    );

    let validation = PublicError::validation(
        request_id,
        ValidationError::single("hostname", ViolationRule::InvalidFormat),
    );
    assert_eq!(
        serde_json::to_value(&validation).unwrap()["details"],
        json!({"kind": "field_violations",
               "violations": [{"field": "hostname", "rule": "invalid_format"}]})
    );

    let approval = PublicError::approval_required(
        request_id,
        ApprovalErrorReason::Pending,
        Some(ApprovalId::parse(SERVER).unwrap()),
    );
    assert_eq!(approval.http_status(), 428);
    assert!(matches!(
        approval.details(),
        Some(ErrorDetails::ApprovalRequired {
            reason: ApprovalErrorReason::Pending,
            ..
        })
    ));

    assert!(
        serde_json::from_value::<PublicError>(json!({
            "code": "internal_sql_error", "message": "x", "request_id": REQUEST, "details": null
        }))
        .is_err()
    );
}

#[test]
fn foreign_and_absent_resources_are_indistinguishable() {
    let request_id = RequestId::parse(REQUEST).unwrap();
    // Both cases go through the same constructor, which takes no resource information.
    let absent = serde_json::to_string(&PublicError::not_found(request_id)).unwrap();
    let foreign = serde_json::to_string(&PublicError::not_found(request_id)).unwrap();
    assert_eq!(absent, foreign);
    assert!(!absent.contains(SERVER));
}

#[test]
fn credential_status_has_no_secret_field() {
    let status = CredentialStatus {
        kind: CredentialKind::SshPrivateKey,
        state: CredentialState::Missing,
        credential_id: None,
        updated_at: None,
    };
    let value = serde_json::to_value(status).unwrap();
    let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["credential_id", "kind", "state", "updated_at"]);
    assert!(!HostTrustSummary::unenrolled().status.permits_connection());
    assert!(!HostTrustStatus::Observed.permits_connection());
    assert!(!HostTrustStatus::Mismatch.permits_connection());
}

#[test]
fn approval_mode_wire_default_and_thresholds() {
    use linux_mcp_contracts::public::inventory::ApprovalMode;
    use linux_mcp_contracts::public::operations::RiskLevel::{Critical, High, Low, Medium};

    assert_eq!(ApprovalMode::default(), ApprovalMode::MediumAndAbove);
    assert!(ApprovalMode::default().is_default());
    assert!(!ApprovalMode::HighAndAbove.is_default());
    assert_eq!(
        serde_json::to_value(ApprovalMode::MediumAndAbove).unwrap(),
        json!("medium_and_above")
    );
    assert_eq!(
        serde_json::to_value(ApprovalMode::HighAndAbove).unwrap(),
        json!("high_and_above")
    );
    assert!(serde_json::from_value::<ApprovalMode>(json!("never")).is_err());
    assert!(serde_json::from_value::<ApprovalMode>(json!("medium")).is_err());

    // Threshold matrix: low never needs approval; critical always does (it is also always denied).
    for (risk, medium_and_above, high_and_above) in [
        (Low, false, false),
        (Medium, true, false),
        (High, true, true),
        (Critical, true, true),
    ] {
        assert_eq!(
            ApprovalMode::MediumAndAbove.requires_approval(risk),
            medium_and_above,
            "{risk:?}"
        );
        assert_eq!(
            ApprovalMode::HighAndAbove.requires_approval(risk),
            high_and_above,
            "{risk:?}"
        );
    }

    // Requests: omitted means the safe default; an update may carry only the mode.
    let create: CreateServerRequest = serde_json::from_value(json!({
        "display_name": "x", "hostname": "h.example", "username": "u",
        "environment": "development"
    }))
    .unwrap();
    assert_eq!(create.approval_mode, ApprovalMode::MediumAndAbove);
    let update: UpdateServerRequest =
        serde_json::from_value(json!({"expected_revision": 2, "approval_mode": "high_and_above"}))
            .unwrap();
    assert!(update.validate().is_ok());
    assert!(update.changes_approval_policy());
    assert!(!update.changes_target());
    let other: UpdateServerRequest =
        serde_json::from_value(json!({"expected_revision": 2, "enabled": false})).unwrap();
    assert!(!other.changes_approval_policy());
}
