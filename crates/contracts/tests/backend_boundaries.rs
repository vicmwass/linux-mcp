//! Step 00-04: trusted identity, authorized targets and credential boundaries.

#![cfg(feature = "backend")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use linux_mcp_contracts::backend::credentials::{
    CredentialError, CredentialLease, CredentialMaterial, CredentialPurpose, CredentialRef,
    MAX_LEASE_MILLIS,
};
use linux_mcp_contracts::backend::identity::{
    AuthKind, IdentityError, Principal, Scope, SessionRef, VerifiedIdentity,
};
use linux_mcp_contracts::backend::target::{
    AuthorizedTarget, HostTrustRef, NetworkTarget, ResolvedTarget, TargetError,
};
use linux_mcp_contracts::public::errors::ErrorCode;
use linux_mcp_contracts::public::ids::{
    CredentialId, HostTrustId, InstallationId, RequestId, ServerId, ServerRevision, Timestamp,
    UserId,
};
use linux_mcp_contracts::public::inventory::{
    ApprovalMode, CredentialKind, HostTrustStatus, Hostname, SshPort, SshUsername,
};

const INSTALL: &str = "aaaaaaaa-0000-4000-8000-000000000001";
const OTHER_INSTALL: &str = "aaaaaaaa-0000-4000-8000-000000000002";
const ALICE: &str = "a11ce000-0000-4000-8000-000000000001";
const BOB: &str = "b0b00000-0000-4000-8000-000000000002";
const ALICE_SERVER: &str = "5e000000-0000-4000-8000-00000000a001";
const BOB_SERVER: &str = "5e000000-0000-4000-8000-00000000b001";

fn ts(millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(1_790_000_000_000 + millis).unwrap()
}

fn principal(user: &str, install: &str, scopes: &[Scope]) -> Principal {
    Principal::from_verified(VerifiedIdentity {
        user_id: UserId::parse(user).unwrap(),
        installation_id: InstallationId::parse(install).unwrap(),
        scopes: scopes.iter().copied().collect::<BTreeSet<_>>(),
        session: SessionRef::new(format!("sess-{user}")).unwrap(),
        auth_kind: AuthKind::McpToken,
        authenticated_at: ts(0),
        expires_at: ts(3_600_000),
    })
    .unwrap()
}

fn credential(owner: &str, server: &str, kind: CredentialKind, n: u8) -> CredentialRef {
    CredentialRef {
        id: CredentialId::parse(&format!("c0000000-0000-4000-8000-0000000000{n:02x}")).unwrap(),
        kind,
        owner: UserId::parse(owner).unwrap(),
        server_id: ServerId::parse(server).unwrap(),
        bound_at_revision: ServerRevision::INITIAL,
        enabled: true,
    }
}

/// Alice and Bob deliberately share the same hostname, port and username.
fn resolved(owner: &str, server: &str, n: u8) -> ResolvedTarget {
    let hostname = Hostname::new("shared.example.internal").unwrap();
    ResolvedTarget {
        owner: UserId::parse(owner).unwrap(),
        installation_id: InstallationId::parse(INSTALL).unwrap(),
        server_id: ServerId::parse(server).unwrap(),
        revision: ServerRevision::new(2).unwrap(),
        enabled: true,
        approval_mode: ApprovalMode::default(),
        network: NetworkTarget {
            hostname: hostname.clone(),
            port: SshPort::DEFAULT,
            username: SshUsername::new("deploy").unwrap(),
        },
        host_trust: Some(HostTrustRef {
            id: HostTrustId::parse(&format!("70000000-0000-4000-8000-0000000000{n:02x}")).unwrap(),
            status: HostTrustStatus::Verified,
            fingerprint: format!("SHA256:fake-fingerprint-{n}"),
            hostname,
            port: SshPort::DEFAULT,
        }),
        credentials: vec![
            credential(owner, server, CredentialKind::SshPrivateKey, n),
            credential(owner, server, CredentialKind::SudoPassword, n + 1),
        ],
    }
}

fn alice() -> Principal {
    principal(
        ALICE,
        INSTALL,
        &[Scope::ServersInspect, Scope::ServersMutate],
    )
}

#[test]
fn principal_requires_valid_lifetime_and_scopes() {
    let result = Principal::from_verified(VerifiedIdentity {
        user_id: UserId::parse(ALICE).unwrap(),
        installation_id: InstallationId::parse(INSTALL).unwrap(),
        scopes: BTreeSet::new(),
        session: SessionRef::new("s").unwrap(),
        auth_kind: AuthKind::BrowserSession,
        authenticated_at: ts(10),
        expires_at: ts(10),
    });
    assert_eq!(result.unwrap_err(), IdentityError::InvalidLifetime);

    let p = alice();
    let install = InstallationId::parse(INSTALL).unwrap();
    assert!(p.authorize(install, Scope::ServersInspect, ts(1)).is_ok());
    assert_eq!(
        p.authorize(install, Scope::AuditRead, ts(1)),
        Err(IdentityError::MissingScope(Scope::AuditRead))
    );
    assert_eq!(
        p.authorize(install, Scope::ServersInspect, ts(3_600_000)),
        Err(IdentityError::Expired)
    );
    assert_eq!(
        p.authorize(
            InstallationId::parse(OTHER_INSTALL).unwrap(),
            Scope::ServersInspect,
            ts(1)
        ),
        Err(IdentityError::WrongInstallation)
    );
}

#[test]
fn scopes_parse_strictly() {
    for scope in Scope::ALL {
        assert_eq!(Scope::parse(scope.as_str()).unwrap(), scope);
    }
    assert!(Scope::parse("servers:*").is_err());
    assert!(Scope::parse("admin").is_err());
    assert!(SessionRef::new("has space").is_err());
    assert!(SessionRef::new("").is_err());
}

#[test]
fn owner_authorizes_own_target() {
    let target = AuthorizedTarget::authorize(
        &alice(),
        resolved(ALICE, ALICE_SERVER, 1),
        Scope::ServersInspect,
        ts(5),
    )
    .unwrap();
    assert_eq!(target.owner(), UserId::parse(ALICE).unwrap());
    assert!(
        target
            .ensure_current(ServerRevision::new(2).unwrap())
            .is_ok()
    );
    let stale = target
        .ensure_current(ServerRevision::new(3).unwrap())
        .unwrap_err();
    assert_eq!(stale.code(), ErrorCode::RevisionConflict);
}

#[test]
fn foreign_target_is_not_found_and_identical_to_absent() {
    let request_id = RequestId::parse(INSTALL).unwrap();
    let err = AuthorizedTarget::authorize(
        &alice(),
        resolved(BOB, BOB_SERVER, 3),
        Scope::ServersInspect,
        ts(5),
    )
    .unwrap_err();
    assert_eq!(err, TargetError::NotFound);
    assert_eq!(
        serde_json::to_string(&err.to_public(request_id)).unwrap(),
        serde_json::to_string(&TargetError::NotFound.to_public(request_id)).unwrap()
    );

    // Same owner ID in another installation is also invisible.
    let mut other = resolved(ALICE, ALICE_SERVER, 1);
    other.installation_id = InstallationId::parse(OTHER_INSTALL).unwrap();
    assert_eq!(
        AuthorizedTarget::authorize(&alice(), other, Scope::ServersInspect, ts(5)).unwrap_err(),
        TargetError::NotFound
    );
}

#[test]
fn ownership_is_checked_before_state_to_avoid_leaks() {
    // A disabled, untrusted foreign server must still look absent, not disabled/untrusted.
    let mut foreign = resolved(BOB, BOB_SERVER, 3);
    foreign.enabled = false;
    foreign.host_trust = None;
    assert_eq!(
        AuthorizedTarget::authorize(&alice(), foreign, Scope::ServersInspect, ts(5)).unwrap_err(),
        TargetError::NotFound
    );
}

#[test]
fn disabled_untrusted_and_mismatched_targets_fail_closed() {
    let mut disabled = resolved(ALICE, ALICE_SERVER, 1);
    disabled.enabled = false;
    assert_eq!(
        AuthorizedTarget::authorize(&alice(), disabled, Scope::ServersInspect, ts(5)).unwrap_err(),
        TargetError::Disabled
    );

    for status in [
        HostTrustStatus::Unenrolled,
        HostTrustStatus::Observed,
        HostTrustStatus::Mismatch,
    ] {
        let mut untrusted = resolved(ALICE, ALICE_SERVER, 1);
        untrusted.host_trust.as_mut().unwrap().status = status;
        assert_eq!(
            AuthorizedTarget::authorize(&alice(), untrusted, Scope::ServersInspect, ts(5))
                .unwrap_err(),
            TargetError::HostTrustNotVerified,
            "{status:?}"
        );
    }

    let mut missing = resolved(ALICE, ALICE_SERVER, 1);
    missing.host_trust = None;
    assert!(AuthorizedTarget::authorize(&alice(), missing, Scope::ServersInspect, ts(5)).is_err());

    // Trust enrolled for another port does not cover this target.
    let mut wrong_port = resolved(ALICE, ALICE_SERVER, 1);
    wrong_port.host_trust.as_mut().unwrap().port = SshPort::new(2222).unwrap();
    assert_eq!(
        AuthorizedTarget::authorize(&alice(), wrong_port, Scope::ServersInspect, ts(5))
            .unwrap_err(),
        TargetError::HostTrustNotVerified
    );

    let reader = principal(ALICE, INSTALL, &[Scope::InventoryRead]);
    let err = AuthorizedTarget::authorize(
        &reader,
        resolved(ALICE, ALICE_SERVER, 1),
        Scope::ServersMutate,
        ts(5),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::Forbidden);
}

#[test]
fn same_hostname_different_owners_have_distinct_cache_keys() {
    let a = AuthorizedTarget::authorize(
        &alice(),
        resolved(ALICE, ALICE_SERVER, 1),
        Scope::ServersInspect,
        ts(5),
    )
    .unwrap();
    let bob = principal(BOB, INSTALL, &[Scope::ServersInspect]);
    let b = AuthorizedTarget::authorize(
        &bob,
        resolved(BOB, BOB_SERVER, 3),
        Scope::ServersInspect,
        ts(5),
    )
    .unwrap();
    assert_eq!(a.resolved().network, b.resolved().network);
    assert_ne!(a.cache_key(), b.cache_key());
}

fn alice_target() -> AuthorizedTarget {
    AuthorizedTarget::authorize(
        &alice(),
        resolved(ALICE, ALICE_SERVER, 1),
        Scope::ServersMutate,
        ts(5),
    )
    .unwrap()
}

#[test]
fn leases_require_bound_credentials() {
    let target = alice_target();
    let key = target.resolved().credentials[0].clone();
    let lease = CredentialLease::issue(
        &target,
        &key,
        CredentialPurpose::SshAuthentication,
        CredentialMaterial::new(
            CredentialKind::SshPrivateKey,
            b"FAKE-SECRET-alice-key".to_vec(),
        ),
        ts(10),
        30_000,
    )
    .unwrap();
    assert_eq!(lease.with_secret(ts(20), |s| s.len()).unwrap(), 21);
    assert_eq!(
        lease.with_secret(ts(30_010), |s| s.len()),
        Err(CredentialError::Expired)
    );

    // Bob's credential cannot be used for Alice's target, even with the same hostname.
    let bobs = credential(BOB, BOB_SERVER, CredentialKind::SshPrivateKey, 3);
    let err = CredentialLease::issue(
        &target,
        &bobs,
        CredentialPurpose::SshAuthentication,
        CredentialMaterial::new(CredentialKind::SshPrivateKey, b"FAKE".to_vec()),
        ts(10),
        1_000,
    )
    .unwrap_err();
    assert_eq!(err, CredentialError::NotBound);

    // A reference with Alice's owner but an unbound ID is also refused.
    let mut forged = key.clone();
    forged.id = CredentialId::parse("c0000000-0000-4000-8000-0000000000ff").unwrap();
    assert_eq!(
        CredentialLease::issue(
            &target,
            &forged,
            CredentialPurpose::SshAuthentication,
            CredentialMaterial::new(CredentialKind::SshPrivateKey, b"FAKE".to_vec()),
            ts(10),
            1_000,
        )
        .unwrap_err(),
        CredentialError::NotBound
    );
}

#[test]
fn leases_check_kind_and_lifetime() {
    let target = alice_target();
    let key = target.resolved().credentials[0].clone();
    let material = || CredentialMaterial::new(CredentialKind::SshPrivateKey, b"FAKE".to_vec());
    assert_eq!(
        CredentialLease::issue(
            &target,
            &key,
            CredentialPurpose::SudoElevation,
            material(),
            ts(0),
            1_000
        )
        .unwrap_err(),
        CredentialError::KindMismatch
    );
    for ttl in [0, MAX_LEASE_MILLIS + 1] {
        assert_eq!(
            CredentialLease::issue(
                &target,
                &key,
                CredentialPurpose::SshAuthentication,
                material(),
                ts(0),
                ttl
            )
            .unwrap_err(),
            CredentialError::InvalidLifetime
        );
    }
}

#[test]
fn debug_output_is_redacted() {
    let target = alice_target();
    let key = target.resolved().credentials[1].clone();
    let lease = CredentialLease::issue(
        &target,
        &key,
        CredentialPurpose::SudoElevation,
        CredentialMaterial::new(
            CredentialKind::SudoPassword,
            b"FAKE-SECRET-hunter2".to_vec(),
        ),
        ts(0),
        1_000,
    )
    .unwrap();
    let text = format!("{lease:?}");
    assert!(text.contains("<redacted>"));
    assert!(!text.contains("hunter2"));
    let material = CredentialMaterial::new(CredentialKind::SudoPassword, b"FAKE-SECRET-x".to_vec());
    assert!(!format!("{material:?}").contains("FAKE-SECRET"));
}

/// Builds a throwaway crate against the public-only feature set and checks that backend
/// items are unreachable. Set `LINUX_MCP_SKIP_PROBES=1` to skip (e.g. no offline cache).
#[test]
fn public_build_cannot_import_backend_modules() {
    if std::env::var_os("LINUX_MCP_SKIP_PROBES").is_some() {
        eprintln!("skipped by LINUX_MCP_SKIP_PROBES");
        return;
    }
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir.parent().unwrap().parent().unwrap();
    let probe = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("public-boundary-probe");
    std::fs::create_dir_all(probe.join("src")).unwrap();
    std::fs::copy(workspace.join("Cargo.lock"), probe.join("Cargo.lock")).unwrap();
    std::fs::write(
        probe.join("Cargo.toml"),
        format!(
            "[package]\nname = \"public-boundary-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
             [workspace]\n\n[dependencies]\nlinux-mcp-contracts = {{ path = {:?}, default-features = false, features = [\"public\"] }}\n",
            crate_dir.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();

    let check = |source: &str| {
        std::fs::write(probe.join("src/lib.rs"), source).unwrap();
        Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args(["check", "--offline", "--quiet"])
            .current_dir(&probe)
            .env("CARGO_TARGET_DIR", probe.join("target"))
            .output()
            .unwrap()
    };

    let ok = check("pub use linux_mcp_contracts::public::ids::ServerId;\n");
    assert!(
        ok.status.success(),
        "public probe must build: {}",
        String::from_utf8_lossy(&ok.stderr)
    );

    for item in [
        "linux_mcp_contracts::backend::identity::Principal",
        "linux_mcp_contracts::backend::credentials::CredentialMaterial",
        "linux_mcp_contracts::backend::target::AuthorizedTarget",
    ] {
        let out = check(&format!("pub use {item};\n"));
        assert!(
            !out.status.success(),
            "{item} must not be reachable from a public build"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("backend"), "unexpected failure: {stderr}");
    }
}
