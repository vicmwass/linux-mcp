//! Step 00-08: MCP compatibility fixtures, legacy tool map, text adapter and v1 extension.

#![cfg(feature = "public")]

use linux_mcp_contracts::public::mcp::*;
use linux_mcp_contracts::public::operations::{RefusalReason, RiskLevel};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/mcp");

fn load(path: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("{FIXTURES}/{path}")).unwrap()).unwrap()
}

fn round_trip<T: DeserializeOwned + Serialize>(path: &str) -> T {
    let value = load(path);
    let parsed: T = serde_json::from_value(value.clone()).unwrap_or_else(|e| panic!("{path}: {e}"));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), value, "{path}");
    parsed
}

fn call_text(name: &str) -> (String, bool) {
    let v = load(&format!("legacy/calls/{name}.response.json"));
    let r = &v["result"];
    (
        r["content"][0]["text"].as_str().unwrap().to_owned(),
        r["isError"].as_bool().unwrap(),
    )
}

#[test]
fn legacy_tool_map_matches_captured_tools_list() {
    let list = load("legacy/tools_list.response.json");
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), LEGACY_TOOLS.len());
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let spec = legacy_tool(name).unwrap_or_else(|| panic!("{name} missing from LEGACY_TOOLS"));
        let schema = &tool["inputSchema"];
        let required: Vec<&str> = schema["required"]
            .as_array()
            .map(|a| a.iter().map(|v| v.as_str().unwrap()).collect())
            .unwrap_or_default();
        assert_eq!(required, spec.required, "{name} required");
        let mut props: Vec<&str> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        props.sort();
        assert_eq!(props, spec.properties, "{name} properties");
        assert_eq!(
            schema["properties"].get("confirm").is_some(),
            spec.has_confirm,
            "{name} confirm"
        );
        assert_eq!(
            schema["properties"].get("server_id").is_some(),
            spec.takes_server_id,
            "{name} server_id"
        );
        assert_eq!(spec.tool_id().as_str(), name);
    }
    let mutating: Vec<&str> = LEGACY_TOOLS
        .iter()
        .filter(|t| t.mutating)
        .map(|t| t.name)
        .collect();
    assert_eq!(
        mutating,
        [
            "create_user",
            "delete_user",
            "disable_user",
            "enable_user",
            "execute_command"
        ]
    );
    assert_eq!(
        legacy_tool("delete_user").unwrap().risk,
        Some(RiskLevel::High)
    );
    assert_eq!(legacy_tool("execute_command").unwrap().risk, None);
    for new in PROPOSED_TOOLS {
        assert!(legacy_tool(new).is_none(), "{new}");
    }
}

#[test]
fn initialize_fixture_shows_current_version_negotiation() {
    let init = load("legacy/initialize.response.json");
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert!(init["result"]["capabilities"].get("tools").is_some());
    assert!(init["result"]["capabilities"].get("experimental").is_none());
}

#[test]
fn legacy_result_envelopes_are_classified_distinctly() {
    use LegacyClassification as C;
    let (t, e) = call_text("execute_command");
    assert_eq!(classify_legacy_text("execute_command", &t, e), C::Failed);
    let (t, e) = call_text("get_user");
    assert_eq!(classify_legacy_text("get_user", &t, e), C::Failed);
    let (t, e) = call_text("delete_user_unconfirmed");
    let c = classify_legacy_text("delete_user", &t, e);
    assert_eq!(c, C::ConfirmationRequired);
    assert_eq!(c.refusal(), Some(RefusalReason::ConfirmationRequired));
    assert!(!c.may_have_run());
    let (t, e) = call_text("missing_argument");
    assert!(e);
    assert_eq!(classify_legacy_text("get_user", &t, e), C::InvalidArguments);
    let unknown = load("legacy/calls/unknown_tool.response.json");
    assert!(unknown.get("result").is_none());
    assert_eq!(unknown["error"]["code"], -32602);

    let cases = [
        (
            "execute_command",
            "Exit code: 0\nSTDOUT:\nuid=0\n\nSTDERR:\n",
            C::ExecCompleted { exit_code: 0 },
        ),
        (
            "execute_command",
            "Exit code: 3\nSTDOUT:\n\nSTDERR:\nboom",
            C::ExecCompleted { exit_code: 3 },
        ),
        (
            "execute_command",
            "This operation requires explicit confirmation.",
            C::ConfirmationRequired,
        ),
        (
            "execute_command",
            "This operation is not permitted.",
            C::Refused,
        ),
        (
            "execute_command",
            "Security policy rejected the command:\nNo supported package manager detected",
            C::Refused,
        ),
        (
            "execute_command",
            "Command rejected:\nunsupported shell construct",
            C::Refused,
        ),
        (
            "execute_command",
            "Command execution failed:\nCommand execution timed out after 60s; remote outcome may be unknown",
            C::PossiblyUnknownOutcome,
        ),
        (
            "execute_command",
            "Command execution failed:\nConnection refused",
            C::Failed,
        ),
        (
            "list_servers",
            "ID: a\nHost: h\nPort: 22\nUser: u\nEnvironment: development",
            C::ToolSucceeded,
        ),
        (
            "list_servers",
            "No Linux servers are configured.",
            C::ToolSucceeded,
        ),
        (
            "create_user",
            "User 'bob' created successfully.",
            C::ToolSucceeded,
        ),
        (
            "delete_user",
            "User 'bob' has been permanently deleted along with their home directory.",
            C::ToolSucceeded,
        ),
        (
            "delete_user",
            "Failed to delete user:\nUser deletion requires explicit confirmation. Set confirm=true.",
            C::ConfirmationRequired,
        ),
        (
            "get_user",
            "Username: bob\nUID: 1001\nGID: 1001\nHome: /home/bob\nShell: /bin/bash",
            C::ToolSucceeded,
        ),
    ];
    for (tool, text, expected) in cases {
        assert_eq!(
            classify_legacy_text(tool, text, false),
            expected,
            "{tool}: {text}"
        );
    }
    assert_eq!(
        classify_legacy_text("execute_command", "something new", false),
        C::Unrecognized
    );
    assert_eq!(
        classify_legacy_text("get_user", "User 'bob' created successfully.", false),
        C::Unrecognized
    );
    assert_eq!(
        classify_legacy_text("execute_command", "Exit code: x\nSTDOUT:\n", false),
        C::Unrecognized
    );
    assert!(C::PossiblyUnknownOutcome.may_have_run());
    assert!(!C::Refused.may_have_run());
}

#[test]
fn legacy_confirm_is_not_approval_evidence() {
    const { assert!(!LEGACY_CONFIRM_IS_APPROVAL_EVIDENCE) };
    for f in [
        "proposed/result.approval_required.json",
        "proposed/submit_operation.params.json",
    ] {
        let text = load(f).to_string();
        assert!(
            !text.contains("confirm") && !text.contains("approved"),
            "{f}"
        );
    }
    assert!(
        serde_json::from_value::<SubmitOperationParams>(json!({
            "operation_id": "0e000000-0000-4000-8000-000000000001", "confirm": true
        }))
        .is_err()
    );
}

#[test]
fn proposed_extension_fixtures_round_trip() {
    let discovery = round_trip::<ExtensionDiscovery>("proposed/discovery.json");
    assert_eq!(discovery, ExtensionDiscovery::v1());
    round_trip::<PrepareOperationParams>("proposed/prepare_operation.params.json");
    round_trip::<SubmitOperationParams>("proposed/submit_operation.params.json");
    round_trip::<OperationStatusParams>("proposed/get_operation_status.params.json");
    let approval = round_trip::<ToolResultEnvelope>("proposed/result.approval_required.json");
    assert!(!approval.is_error());
    for (f, is_error) in [
        ("proposed/result.refused.json", true),
        ("proposed/result.succeeded_nonzero_exit.json", false),
        ("proposed/result.unknown_after_submission.json", true),
        ("proposed/result.error_not_found.json", true),
    ] {
        assert_eq!(
            round_trip::<ToolResultEnvelope>(f).is_error(),
            is_error,
            "{f}"
        );
    }
    let refused = load("proposed/result.refused.json");
    assert_eq!(refused["result"]["kind"], "outcome");
    assert_eq!(refused["result"]["outcome"]["status"], "refused");
}

#[test]
fn incompatible_inputs_are_rejected_not_guessed() {
    let mut seen = 0;
    for entry in std::fs::read_dir(format!("{FIXTURES}/incompatible")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let value = load(&format!("incompatible/{name}"));
        assert!(
            serde_json::from_value::<ToolResultEnvelope>(value).is_err(),
            "{name} must be rejected"
        );
        seen += 1;
    }
    assert_eq!(seen, 5);
    assert!(McpSchemaVersion::parse("linux-mcp/v2").is_err());
    assert!(McpSchemaVersion::parse("").is_err());
    assert_eq!(
        McpSchemaVersion::parse("linux-mcp/v1").unwrap(),
        McpSchemaVersion::V1
    );
    assert_eq!(
        McpSchemaVersion::parse("linux-mcp/legacy-1")
            .unwrap()
            .as_str(),
        LEGACY_SCHEMA_VERSION
    );
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = vec![];
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else if p.extension().is_some_and(|x| x == "json") {
            out.push(p);
        }
    }
    out
}

#[test]
fn legacy_fixtures_contain_no_server_inventory() {
    for entry in walk(std::path::Path::new(&format!("{FIXTURES}/legacy"))) {
        let text = std::fs::read_to_string(&entry).unwrap();
        for needle in [
            "ubuntu-01",
            "linux-mint",
            "10.0.",
            "keys/",
            "sudo_password_file",
        ] {
            assert!(!text.contains(needle), "{entry:?} leaks {needle}");
        }
    }
}

#[cfg(feature = "schema")]
mod schemas {
    use super::*;
    use schemars::{JsonSchema, schema_for};

    const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/schemas/mcp");

    fn check<T: JsonSchema>(name: &str) {
        let mut text = serde_json::to_string_pretty(&schema_for!(T)).unwrap();
        text.push('\n');
        let path = format!("{DIR}/{name}.schema.json");
        if std::env::var_os("LINUX_MCP_UPDATE_SCHEMAS").is_some()
            || !std::path::Path::new(&path).exists()
        {
            std::fs::write(&path, &text).unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(&path)
                .unwrap()
                .replace("\r\n", "\n"),
            text,
            "{name} drifted; set LINUX_MCP_UPDATE_SCHEMAS=1"
        );
    }

    #[test]
    fn extension_schemas_match_rust_types() {
        check::<ToolResultEnvelope>("ToolResultEnvelope");
        check::<ApprovalRequiredResult>("ApprovalRequiredResult");
        check::<SubmitOperationParams>("SubmitOperationParams");
        check::<OperationStatusParams>("OperationStatusParams");
        check::<PrepareOperationParams>("PrepareOperationParams");
        check::<ExtensionDiscovery>("ExtensionDiscovery");
    }
}
