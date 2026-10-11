//! Step 00-06: management API contracts, route table, fixtures and generated schemas.

#![cfg(feature = "public")]

use std::collections::BTreeSet;

use linux_mcp_contracts::public::api::*;
use linux_mcp_contracts::public::errors::ErrorCode;
use linux_mcp_contracts::public::inventory::{CreateServerRequest, UpdateServerRequest};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/management");

fn load(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("{FIXTURES}/{name}")).unwrap()).unwrap()
}

fn round_trip<T: DeserializeOwned + Serialize>(name: &str) -> T {
    let value = load(name);
    let parsed: T = serde_json::from_value(value.clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(
        serde_json::to_value(&parsed).unwrap(),
        value,
        "{name} round trip"
    );
    parsed
}

#[test]
fn fixtures_round_trip() {
    round_trip::<CreateServerRequest>("create_server.request.json");
    round_trip::<UpdateServerRequest>("update_server.request.json");
    round_trip::<PutCredentialRequest>("put_credential.request.json");
    round_trip::<CredentialWriteResponse>("put_credential.response.json");
    round_trip::<HostKeyObservation>("host_key_observation.response.json");
    let enroll = round_trip::<EnrollHostKeyRequest>("enroll_host_key.request.json");
    assert!(enroll.validate_fingerprint().is_ok());
    round_trip::<CreateApprovalRequest>("create_approval.request.json");
    round_trip::<ApprovalDecisionRequest>("decide_approval.request.json");
    round_trip::<AuditRecord>("audit_record.response.json");
    let authz = round_trip::<AuditRecord>("audit_authz_failure.response.json");
    assert!(authz.server_id.is_none() && authz.tool.is_none());
    for (name, code) in [
        ("error.not_found.json", ErrorCode::NotFound),
        ("error.revision_conflict.json", ErrorCode::RevisionConflict),
        ("error.validation.json", ErrorCode::Validation),
        ("error.approval_required.json", ErrorCode::ApprovalRequired),
    ] {
        let envelope = round_trip::<ErrorEnvelope>(name);
        assert_eq!(envelope.error.code(), code);
    }
}

#[test]
fn secrets_are_write_only_and_redacted() {
    let request: PutCredentialRequest =
        serde_json::from_value(load("put_credential.request.json")).unwrap();
    assert!(!format!("{request:?}").contains("FAKE-SECRET"));
    for name in [
        "put_credential.response.json",
        "host_key_observation.response.json",
        "audit_record.response.json",
    ] {
        let text = load(name).to_string();
        assert!(
            !text.contains("FAKE-SECRET")
                && !text.contains("\"password\"")
                && !text.contains("private_key"),
            "{name}"
        );
    }
    assert!(serde_json::from_value::<WriteOnlySecret>(json!("")).is_err());
    assert!(serde_json::from_value::<WriteOnlySecret>(json!("a\u{0}b")).is_err());
    assert!(
        serde_json::from_value::<CredentialInput>(
            json!({"type": "sudo_password", "password": "x", "extra": 1})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<CredentialInput>(json!({"type": "sudo_passwordless"})).is_ok()
    );
}

#[test]
fn requests_reject_owner_and_unknown_fields() {
    let owner = "a11ce000-0000-4000-8000-000000000001";
    let mut v = load("create_approval.request.json");
    v["owner_user_id"] = json!(owner);
    assert!(serde_json::from_value::<CreateApprovalRequest>(v).is_err());
    let mut v = load("put_credential.request.json");
    v["owner_user_id"] = json!(owner);
    assert!(serde_json::from_value::<PutCredentialRequest>(v).is_err());
    let mut v = load("create_approval.request.json");
    v["operation"]["approved"] = json!(true);
    assert!(
        serde_json::from_value::<CreateApprovalRequest>(v).is_err(),
        "no approval boolean on requests"
    );
    let mut v = load("decide_approval.request.json");
    v["decision"] = json!("maybe");
    assert!(serde_json::from_value::<ApprovalDecisionRequest>(v).is_err());
    let mut v = load("create_approval.request.json");
    v["operation"]["arguments"]["password"] = json!("x");
    assert!(serde_json::from_value::<CreateApprovalRequest>(v).is_err());
}

#[test]
fn fingerprint_validation() {
    let ok = "SHA256:AAAAexamplefingerprintAAAAAAAAAAAAAAAAAAAAA";
    let mk = |f: &str| EnrollHostKeyRequest {
        expected_revision: 1.try_into().unwrap(),
        fingerprint: f.into(),
    };
    assert!(mk(ok).validate_fingerprint().is_ok());
    let too_long = format!("{ok}A");
    for bad in [
        "",
        "MD5:aa:bb",
        "SHA256:short",
        "SHA256:AAAAexamplefingerprintAAAAAAAAAAAAAAAAAAAAAA=",
        &too_long,
    ] {
        assert!(mk(bad).validate_fingerprint().is_err(), "{bad}");
    }
}

#[test]
fn route_table_is_consistent_and_complete() {
    let mut ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for r in ROUTES {
        assert!(ids.insert(r.id), "duplicate id {}", r.id);
        assert!(
            keys.insert((format!("{:?}", r.method), r.path)),
            "duplicate route {}",
            r.path
        );
        assert!(r.path.starts_with(API_BASE));
        assert!(!r.scopes.is_empty());
        assert!(
            r.scopes.iter().all(|s| linux_mcp_contracts_scope_ok(s)),
            "{}",
            r.id
        );
        assert!(r.errors.contains(&ErrorCode::Unauthenticated));
        assert!(
            !r.errors.contains(&ErrorCode::OutcomeUnknown),
            "{}: management routes never return outcome_unknown",
            r.id
        );
        if matches!(r.method, Method::Patch | Method::Put) {
            assert!(r.request.is_some(), "{}", r.id);
            assert!(r.errors.contains(&ErrorCode::RevisionConflict), "{}", r.id);
        }
        if r.path.contains("{server_id}") && !matches!(r.method, Method::Get) {
            assert!(r.errors.contains(&ErrorCode::RevisionConflict), "{}", r.id);
        }
        if r.path.contains('{') {
            assert!(
                r.errors.contains(&ErrorCode::NotFound),
                "{} must map foreign/absent IDs to 404",
                r.id
            );
        }
        assert!(matches!(r.success_status, 200 | 201 | 204));
        assert_eq!(r.response.is_none(), r.success_status == 204);
    }
    for id in [
        "list_servers",
        "create_server",
        "get_server",
        "update_server",
        "delete_server",
        "put_credential",
        "delete_credential",
        "observe_host_key",
        "enroll_host_key",
        "revoke_host_key",
        "create_approval",
        "list_approvals",
        "get_approval",
        "decide_approval",
        "list_audit",
        "get_audit_record",
    ] {
        assert!(route(id).is_some(), "missing route {id}");
    }
    assert_eq!(
        route("decide_approval").unwrap().auth,
        RouteAuth::InteractiveSessionWithCsrf
    );
    assert!(
        ROUTES
            .iter()
            .all(|r| !r.path.contains("execute") && !r.path.contains("consume")),
        "management API must not be a second execution path"
    );
    assert!(
        ROUTES
            .iter()
            .filter(|r| r.path.starts_with("/api/v1/audit"))
            .all(|r| matches!(r.method, Method::Get)),
        "audit is read-only"
    );
}

fn linux_mcp_contracts_scope_ok(scope: &str) -> bool {
    [
        "inventory:read",
        "inventory:write",
        "credentials:write",
        "servers:inspect",
        "servers:mutate",
        "approvals:request",
        "approvals:decide",
        "audit:read",
    ]
    .contains(&scope)
}

#[cfg(feature = "schema")]
mod schemas {
    use super::*;
    use linux_mcp_contracts::public::{
        errors::PublicError, inventory::ServerSummary, operations::ApprovalSummary,
        pagination::PageRequest,
    };
    use schemars::{JsonSchema, schema_for};

    const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/schemas/management");

    fn schema_text<T: JsonSchema>() -> String {
        let mut text = serde_json::to_string_pretty(&schema_for!(T)).unwrap();
        text.push('\n');
        text
    }

    fn check(name: &str, text: String) {
        let path = format!("{DIR}/{name}.schema.json");
        if std::env::var_os("LINUX_MCP_UPDATE_SCHEMAS").is_some()
            || !std::path::Path::new(&path).exists()
        {
            std::fs::write(&path, &text).unwrap();
        }
        let committed = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            committed.replace("\r\n", "\n"),
            text,
            "{name} schema drifted; rerun with LINUX_MCP_UPDATE_SCHEMAS=1"
        );
    }

    macro_rules! schemas {
        ($($t:ty => $n:literal),* $(,)?) => {
            #[test]
            fn schemas_match_rust_types() {
                $( check($n, schema_text::<$t>()); )*
            }

            #[test]
            fn every_route_schema_name_has_a_file() {
                let known: BTreeSet<&str> = [$($n),*].into_iter().collect();
                for r in ROUTES {
                    for name in [r.query, r.request, r.response].into_iter().flatten() {
                        assert!(known.contains(name), "{}: schema {name} not generated", r.id);
                    }
                }
            }
        };
    }

    schemas! {
        CreateServerRequest => "CreateServerRequest",
        UpdateServerRequest => "UpdateServerRequest",
        ServerSummary => "ServerSummary",
        ServerPage => "ServerPage",
        PageRequest => "PageRequest",
        ExpectedRevisionQuery => "ExpectedRevisionQuery",
        PutCredentialRequest => "PutCredentialRequest",
        CredentialWriteResponse => "CredentialWriteResponse",
        HostKeyObservation => "HostKeyObservation",
        EnrollHostKeyRequest => "EnrollHostKeyRequest",
        CreateApprovalRequest => "CreateApprovalRequest",
        ApprovalDecisionRequest => "ApprovalDecisionRequest",
        ApprovalQuery => "ApprovalQuery",
        ApprovalSummary => "ApprovalSummary",
        ApprovalPage => "ApprovalPage",
        AuditQuery => "AuditQuery",
        AuditRecord => "AuditRecord",
        AuditPage => "AuditPage",
        ErrorEnvelope => "ErrorEnvelope",
        PublicError => "PublicError",
    }

    #[test]
    fn write_only_secret_is_marked_in_schema() {
        assert!(schema_text::<PutCredentialRequest>().contains("\"writeOnly\": true"));
    }
}
