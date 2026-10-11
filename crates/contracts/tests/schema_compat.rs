//! Step 00-10: wire schemas agree with the Rust types and every fixture.
//!
//! Committed schemas (`schemas/`) are generated from the Rust types by the module tests and
//! compared here with fixtures through a small JSON Schema validator, covering exactly the
//! keywords `schemars` emits for this crate. Incompatible fixtures must be rejected by the
//! schemas as well as by the Rust types, so the two cannot drift apart silently.

#![cfg(feature = "schema")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use linux_mcp_contracts::{
    CANONICALIZATION_VERSION, CONTRACT_VERSION, MANAGEMENT_API_VERSION, MCP_EXTENSION_VERSION,
    MCP_LEGACY_VERSION,
};
use serde_json::Value;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn path(rel: &str) -> PathBuf {
    Path::new(ROOT).join(rel)
}

fn read_json(file: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(file).unwrap_or_else(|e| panic!("{file:?}: {e}")))
        .unwrap_or_else(|e| panic!("{file:?}: {e}"))
}

fn schema(dir: &str, name: &str) -> Value {
    read_json(&path(&format!("schemas/{dir}/{name}.schema.json")))
}

// ---------------------------------------------------------------------------------------
// Minimal JSON Schema validator (draft 2020-12 subset emitted by schemars 1.x)
// ---------------------------------------------------------------------------------------

fn resolve<'a>(root: &'a Value, reference: &str) -> &'a Value {
    let pointer = reference.strip_prefix('#').expect("local $ref");
    root.pointer(pointer)
        .unwrap_or_else(|| panic!("unresolved $ref {reference}"))
}

fn type_matches(kind: &str, value: &Value) -> bool {
    match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        other => panic!("unsupported type {other}"),
    }
}

/// Returns the first violation, if any. Unsupported keywords panic so drift is noticed.
fn validate(root: &Value, schema: &Value, value: &Value, at: &str) -> Result<(), String> {
    let Some(object) = schema.as_object() else {
        return if schema == &Value::Bool(true) {
            Ok(())
        } else {
            Err(format!("{at}: schema is false"))
        };
    };
    for key in object.keys() {
        const KNOWN: &[&str] = &[
            "$schema",
            "$ref",
            "$defs",
            "title",
            "description",
            "type",
            "properties",
            "required",
            "additionalProperties",
            "items",
            "oneOf",
            "anyOf",
            "allOf",
            "enum",
            "const",
            "minimum",
            "maximum",
            "minLength",
            "maxLength",
            "pattern",
            "format",
            "default",
            "writeOnly",
        ];
        assert!(
            KNOWN.contains(&key.as_str()),
            "validator does not support `{key}` at {at}"
        );
    }
    if let Some(reference) = object.get("$ref") {
        validate(root, resolve(root, reference.as_str().unwrap()), value, at)?;
    }
    if let Some(types) = object.get("type") {
        let ok = match types {
            Value::String(t) => type_matches(t, value),
            Value::Array(ts) => ts.iter().any(|t| type_matches(t.as_str().unwrap(), value)),
            _ => panic!("bad type"),
        };
        if !ok {
            return Err(format!("{at}: expected type {types}, got {value}"));
        }
    }
    if let Some(c) = object.get("const")
        && c != value
    {
        return Err(format!("{at}: expected const {c}, got {value}"));
    }
    if let Some(Value::Array(options)) = object.get("enum")
        && !options.contains(value)
    {
        return Err(format!("{at}: {value} not in enum"));
    }
    if let (Some(n), Some(min)) = (
        value.as_f64(),
        object.get("minimum").and_then(Value::as_f64),
    ) && n < min
    {
        return Err(format!("{at}: {n} < minimum {min}"));
    }
    if let (Some(n), Some(max)) = (
        value.as_f64(),
        object.get("maximum").and_then(Value::as_f64),
    ) && n > max
    {
        return Err(format!("{at}: {n} > maximum {max}"));
    }
    if let Some(s) = value.as_str() {
        let len = s.chars().count() as u64;
        if let Some(min) = object.get("minLength").and_then(Value::as_u64)
            && len < min
        {
            return Err(format!("{at}: shorter than {min}"));
        }
        if let Some(max) = object.get("maxLength").and_then(Value::as_u64)
            && len > max
        {
            return Err(format!("{at}: longer than {max}"));
        }
    }
    if let Some(map) = value.as_object() {
        let props = object.get("properties").and_then(Value::as_object);
        if let Some(Value::Array(required)) = object.get("required") {
            for r in required {
                if !map.contains_key(r.as_str().unwrap()) {
                    return Err(format!("{at}: missing required `{}`", r.as_str().unwrap()));
                }
            }
        }
        for (key, member) in map {
            match props.and_then(|p| p.get(key)) {
                Some(sub) => validate(root, sub, member, &format!("{at}.{key}"))?,
                None => {
                    if object.get("additionalProperties") == Some(&Value::Bool(false)) {
                        return Err(format!("{at}: unexpected property `{key}`"));
                    }
                }
            }
        }
    }
    if let (Some(items), Some(sub)) = (value.as_array(), object.get("items")) {
        for (i, item) in items.iter().enumerate() {
            validate(root, sub, item, &format!("{at}[{i}]"))?;
        }
    }
    if let Some(Value::Array(subs)) = object.get("allOf") {
        for sub in subs {
            validate(root, sub, value, at)?;
        }
    }
    if let Some(Value::Array(subs)) = object.get("anyOf")
        && !subs.iter().any(|s| validate(root, s, value, at).is_ok())
    {
        return Err(format!("{at}: matches none of anyOf"));
    }
    if let Some(Value::Array(subs)) = object.get("oneOf") {
        let matching = subs
            .iter()
            .filter(|s| validate(root, s, value, at).is_ok())
            .count();
        if matching != 1 {
            return Err(format!("{at}: matches {matching} of oneOf (expected 1)"));
        }
    }
    Ok(())
}

fn conforms(schema_value: &Value, instance: &Value) -> Result<(), String> {
    validate(schema_value, schema_value, instance, "$")
}

// ---------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------

#[test]
fn pinned_versions() {
    assert_eq!(CONTRACT_VERSION, "1.0.0");
    assert_eq!(
        env!("CARGO_PKG_VERSION"),
        CONTRACT_VERSION,
        "crate version tracks the contract version"
    );
    assert_eq!(MANAGEMENT_API_VERSION, "v1");
    assert_eq!(MCP_EXTENSION_VERSION, "linux-mcp/v1");
    assert_eq!(MCP_LEGACY_VERSION, "linux-mcp/legacy-1");
    assert_eq!(CANONICALIZATION_VERSION, "lmcp-canon-v1");
    // The route table, MCP types and schemas use the same pinned values.
    use linux_mcp_contracts::public::api::{API_BASE, ROUTES};
    assert_eq!(API_BASE, format!("/api/{MANAGEMENT_API_VERSION}"));
    assert!(ROUTES.iter().all(|r| r.path.starts_with(API_BASE)));
    assert_eq!(
        linux_mcp_contracts::public::mcp::EXTENSION_SCHEMA_VERSION,
        MCP_EXTENSION_VERSION
    );
    assert_eq!(
        linux_mcp_contracts::public::mcp::LEGACY_SCHEMA_VERSION,
        MCP_LEGACY_VERSION
    );
    let envelope = schema("mcp", "ToolResultEnvelope");
    assert_eq!(
        envelope.pointer("/$defs/ExtensionVersionV1/const").unwrap(),
        MCP_EXTENSION_VERSION
    );
}

fn schema_names(dir: &str) -> BTreeSet<String> {
    std::fs::read_dir(path(&format!("schemas/{dir}")))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n != ".gitkeep")
        .map(|n| {
            n.strip_suffix(".schema.json")
                .unwrap_or_else(|| panic!("stray file {n}"))
                .to_owned()
        })
        .collect()
}

#[test]
fn schema_sets_are_exactly_the_published_surface() {
    let management: BTreeSet<String> = [
        "ApprovalDecisionRequest",
        "ApprovalPage",
        "ApprovalQuery",
        "ApprovalSummary",
        "AuditPage",
        "AuditQuery",
        "AuditRecord",
        "CreateApprovalRequest",
        "CreateServerRequest",
        "CredentialWriteResponse",
        "EnrollHostKeyRequest",
        "ErrorEnvelope",
        "ExpectedRevisionQuery",
        "HostKeyObservation",
        "PageRequest",
        "PublicError",
        "PutCredentialRequest",
        "ServerPage",
        "ServerSummary",
        "UpdateServerRequest",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_eq!(
        schema_names("management"),
        management,
        "stale or missing management schema"
    );
    let mcp: BTreeSet<String> = [
        "ApprovalRequiredResult",
        "ExtensionDiscovery",
        "OperationStatusParams",
        "PrepareOperationParams",
        "SubmitOperationParams",
        "ToolResultEnvelope",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_eq!(schema_names("mcp"), mcp, "stale or missing mcp schema");
    // Every route refers to a published schema.
    for route in linux_mcp_contracts::public::api::ROUTES {
        for name in [route.query, route.request, route.response]
            .into_iter()
            .flatten()
        {
            assert!(management.contains(name), "{}: {name}", route.id);
        }
    }
}

#[test]
fn all_schemas_are_draft_2020_12_documents() {
    for dir in ["management", "mcp"] {
        for name in schema_names(dir) {
            let s = schema(dir, &name);
            assert_eq!(
                s["$schema"], "https://json-schema.org/draft/2020-12/schema",
                "{dir}/{name}"
            );
            // Aliases (`ServerPage`, `PrepareOperationParams`) keep the title of the aliased type.
            assert!(
                s["title"].as_str().is_some_and(|t| !t.is_empty()),
                "{dir}/{name}"
            );
        }
    }
}

fn fixtures(dir: &str, pattern_suffix: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(path(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(pattern_suffix)
        })
        .collect();
    files.sort();
    files
}

#[test]
fn management_fixtures_validate_against_their_schemas() {
    let table: &[(&str, &str)] = &[
        ("create_server.request.json", "CreateServerRequest"),
        ("update_server.request.json", "UpdateServerRequest"),
        ("put_credential.request.json", "PutCredentialRequest"),
        ("put_credential.response.json", "CredentialWriteResponse"),
        ("host_key_observation.response.json", "HostKeyObservation"),
        ("enroll_host_key.request.json", "EnrollHostKeyRequest"),
        ("create_approval.request.json", "CreateApprovalRequest"),
        ("decide_approval.request.json", "ApprovalDecisionRequest"),
        ("audit_record.response.json", "AuditRecord"),
        ("audit_authz_failure.response.json", "AuditRecord"),
        ("error.not_found.json", "ErrorEnvelope"),
        ("error.revision_conflict.json", "ErrorEnvelope"),
        ("error.validation.json", "ErrorEnvelope"),
        ("error.approval_required.json", "ErrorEnvelope"),
    ];
    let covered: BTreeSet<&str> = table.iter().map(|(f, _)| *f).collect();
    for file in fixtures("fixtures/management", ".json") {
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        assert!(
            covered.contains(name.as_str()),
            "fixture {name} has no schema mapping"
        );
    }
    for (file, name) in table {
        let instance = read_json(&path(&format!("fixtures/management/{file}")));
        conforms(&schema("management", name), &instance)
            .unwrap_or_else(|e| panic!("{file} vs {name}: {e}"));
    }
}

#[test]
fn mcp_fixtures_validate_and_incompatible_ones_do_not() {
    let discovery = schema("mcp", "ExtensionDiscovery");
    let mut checked = 0;
    for file in fixtures("fixtures/mcp/proposed", ".json") {
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        let schema_name = match name.as_str() {
            "discovery.json" => "ExtensionDiscovery",
            "prepare_operation.params.json" => "PrepareOperationParams",
            "submit_operation.params.json" => "SubmitOperationParams",
            "get_operation_status.params.json" => "OperationStatusParams",
            n if n.starts_with("result.") => "ToolResultEnvelope",
            other => panic!("no schema mapping for {other}"),
        };
        conforms(&schema("mcp", schema_name), &read_json(&file))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        checked += 1;
    }
    assert_eq!(checked, 9);
    let _ = discovery;

    // Everything the Rust types reject, the schema rejects too.
    let envelope = schema("mcp", "ToolResultEnvelope");
    let incompatible = fixtures("fixtures/mcp/incompatible", ".json");
    assert_eq!(incompatible.len(), 5);
    for file in incompatible {
        assert!(
            conforms(&envelope, &read_json(&file)).is_err(),
            "{file:?} must violate the schema"
        );
    }
}

#[test]
fn operation_fixtures_validate_against_generated_schemas() {
    use linux_mcp_contracts::public::operations::{OperationIntent, OperationOutcome};
    use schemars::schema_for;
    let intent_schema = serde_json::to_value(schema_for!(OperationIntent)).unwrap();
    conforms(
        &intent_schema,
        &read_json(&path("fixtures/operations/intent.json")),
    )
    .unwrap();
    let outcome_schema = serde_json::to_value(schema_for!(OperationOutcome)).unwrap();
    let outcomes = read_json(&path("fixtures/operations/outcomes.json"));
    for (i, outcome) in outcomes.as_array().unwrap().iter().enumerate() {
        conforms(&outcome_schema, outcome).unwrap_or_else(|e| panic!("outcome {i}: {e}"));
    }
    // An unknown outcome carrying an exit status violates the schema (and the Rust type).
    let bad = serde_json::json!({
        "status": "unknown_after_submission", "cause": "disconnected",
        "partial_stdout": null, "partial_stderr": null,
        "exit_status": {"kind": "code", "code": 1}
    });
    assert!(conforms(&outcome_schema, &bad).is_err());
}

#[test]
fn requests_with_owner_fields_violate_schemas() {
    let owner = serde_json::json!("a11ce000-0000-4000-8000-000000000001");
    for (name, fixture) in [
        ("CreateServerRequest", "create_server.request.json"),
        ("UpdateServerRequest", "update_server.request.json"),
        ("PutCredentialRequest", "put_credential.request.json"),
        ("CreateApprovalRequest", "create_approval.request.json"),
        ("ApprovalDecisionRequest", "decide_approval.request.json"),
    ] {
        let s = schema("management", name);
        let mut instance = read_json(&path(&format!("fixtures/management/{fixture}")));
        assert!(conforms(&s, &instance).is_ok());
        instance["owner_user_id"] = owner.clone();
        assert!(
            conforms(&s, &instance).is_err(),
            "{name} must reject owner_user_id"
        );
    }
}

#[test]
fn response_schemas_expose_no_secret_fields() {
    fn property_names(value: &Value, out: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                if let Some(Value::Object(props)) = map.get("properties") {
                    out.extend(props.keys().cloned());
                }
                map.values().for_each(|v| property_names(v, out));
            }
            Value::Array(items) => items.iter().for_each(|v| property_names(v, out)),
            _ => {}
        }
    }
    for name in [
        "ServerSummary",
        "ServerPage",
        "CredentialWriteResponse",
        "HostKeyObservation",
        "ApprovalSummary",
        "ApprovalPage",
        "AuditRecord",
        "AuditPage",
        "ErrorEnvelope",
        "PublicError",
    ] {
        let mut props = BTreeSet::new();
        property_names(&schema("management", name), &mut props);
        for p in &props {
            assert!(
                !linux_mcp_contracts::canonical::is_sensitive_key(p)
                    || matches!(p.as_str(), "credential" | "credential_id" | "credentials"),
                "{name} exposes sensitive-looking property `{p}`"
            );
            assert!(!p.contains("owner") || p == "owner", "{name}: {p}");
        }
        assert!(
            !props.contains("password") && !props.contains("private_key"),
            "{name}"
        );
    }
    // Write-only secrets appear only in request schemas, marked writeOnly.
    let put = serde_json::to_string(&schema("management", "PutCredentialRequest")).unwrap();
    assert!(put.contains("\"writeOnly\":true"));
    for name in ["CredentialWriteResponse", "ServerSummary", "AuditRecord"] {
        assert!(
            !serde_json::to_string(&schema("management", name))
                .unwrap()
                .contains("writeOnly"),
            "{name}"
        );
    }
}

fn cargo_tree(args: &[&str]) -> String {
    let output =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args([
                "tree",
                "--offline",
                "--locked",
                "-p",
                "linux-mcp-contracts",
                "-e",
                "normal",
                "--prefix",
                "none",
            ])
            .args(args)
            .current_dir(ROOT)
            .output()
            .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// Set `LINUX_MCP_SKIP_PROBES=1` to skip tests that spawn cargo.
#[test]
fn dependency_graph_has_no_backend_leakage_or_reverse_dependencies() {
    if std::env::var_os("LINUX_MCP_SKIP_PROBES").is_some() {
        return;
    }
    let names = |tree: &str| -> BTreeSet<String> {
        tree.lines()
            .filter_map(|l| l.split_whitespace().next())
            .map(String::from)
            .collect()
    };
    let public = names(&cargo_tree(&[
        "--no-default-features",
        "--features",
        "public",
    ]));
    for forbidden in [
        "async-trait",
        "zeroize",
        "schemars",
        "getrandom",
        "tokio",
        "rmcp",
        "russh",
        "linux-mcp",
    ] {
        assert!(
            !public.contains(forbidden),
            "public build pulls in {forbidden}: {public:?}"
        );
    }
    let all = names(&cargo_tree(&["--all-features"]));
    for forbidden in [
        "tokio",
        "rmcp",
        "russh",
        "sqlx",
        "axum",
        "hyper",
        "reqwest",
        "linux-mcp",
    ] {
        assert!(!all.contains(forbidden), "contracts depend on {forbidden}");
    }
    // The root application does not depend on the contracts crate during Spec 00.
    let inverse = cargo_tree(&["--invert", "linux-mcp-contracts"]);
    assert_eq!(
        inverse.lines().filter(|l| !l.trim().is_empty()).count(),
        1,
        "unexpected reverse deps:\n{inverse}"
    );
}
