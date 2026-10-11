//! Alice/Bob scenario seeds, the demo entry point and a call-sequence driver.
//!
//! The seed contains no real secret, host or user; every secret is a `FAKE-SECRET-*` marker.
//! Alice and Bob own servers that share the same hostname, port and SSH username but have
//! distinct IDs, credentials and host-trust records, so any code that keys state by hostname
//! alone leaks across owners.
//!
//! [`Driver`] is the reference implementation of the call sequence documented in
//! `backend::services` (resolve → intent → evaluate → refuse | request approval | consume →
//! begin audit → submit). Protocol adapters (Spec 04/08) copy this order.

use crate::backend::credentials::CredentialRef;
use crate::backend::identity::{AuthKind, Principal, Scope};
use crate::backend::operation_authority::ExecutionAuthority;
use crate::backend::services::*;
use crate::public::api::{ApprovalDecision, ApprovalDecisionRequest};
use crate::public::errors::PublicError;
use crate::public::ids::*;
use crate::public::inventory::{ApprovalMode, CredentialKind, Environment};
use crate::public::mcp::legacy_tool;
use crate::public::operations::*;
use serde_json::Value;

use super::memory::{ExecScript, SeedServer, World};

pub const INSTALLATION: &str = "aaaaaaaa-0000-4000-8000-000000000001";
pub const OTHER_INSTALLATION: &str = "aaaaaaaa-0000-4000-8000-000000000002";
pub const ALICE_ID: &str = "a11ce000-0000-4000-8000-000000000001";
pub const BOB_ID: &str = "b0b00000-0000-4000-8000-000000000002";
pub const SHARED_HOSTNAME: &str = "shared.example.internal";

const ALICE_FINGERPRINT: &str = "SHA256:AliceHostKeyFakeFakeFakeFakeFakeFakeFak";
const BOB_FINGERPRINT: &str = "SHA256:BobHostKeyFakeFakeFakeFakeFakeFakeFakeFa";

/// What a person presents to the system: a token (agent) and a browser session (human).
#[derive(Debug, Clone)]
pub struct Actor {
    pub user: UserId,
    pub mcp_token: String,
    pub session_cookie: String,
    pub csrf: String,
}

#[derive(Debug, Clone)]
pub struct DemoServers {
    /// Alice's server at `shared.example.internal`, host trust verified, revision 2.
    pub alice_shared: ServerId,
    /// Bob's server at the same hostname, port and username, with his own credentials.
    pub bob_shared: ServerId,
    /// Alice's second server: no host trust enrolled.
    pub alice_untrusted: ServerId,
}

#[derive(Debug, Clone)]
pub struct Demo {
    pub world: World,
    pub alice: Actor,
    pub bob: Actor,
    pub servers: DemoServers,
    pub alice_ssh_key: CredentialRef,
    pub bob_ssh_key: CredentialRef,
}

const AGENT_SCOPES: &[Scope] = &[
    Scope::InventoryRead,
    Scope::ServersInspect,
    Scope::ServersMutate,
    Scope::ApprovalsRequest,
    Scope::AuditRead,
];

const BROWSER_SCOPES: &[Scope] = &[
    Scope::InventoryRead,
    Scope::InventoryWrite,
    Scope::CredentialsWrite,
    Scope::ServersInspect,
    Scope::ServersMutate,
    Scope::ApprovalsRequest,
    Scope::ApprovalsDecide,
    Scope::AuditRead,
];

fn actor(world: &World, user: UserId) -> Actor {
    let mcp_token = world.issue_token(user, AGENT_SCOPES, AuthKind::McpToken, 3600);
    let session_cookie = world.issue_token(user, BROWSER_SCOPES, AuthKind::BrowserSession, 3600);
    let csrf = world.csrf_for(&session_cookie).expect("session exists");
    Actor {
        user,
        mcp_token,
        session_cookie,
        csrf,
    }
}

/// Seeds Alice and Bob into an empty (or reset) world.
pub fn seed_demo(world: &World) -> Demo {
    let alice_id = UserId::parse(ALICE_ID).unwrap();
    let bob_id = UserId::parse(BOB_ID).unwrap();
    let alice_shared = ServerId::parse("5e000000-0000-4000-8000-00000000a001").unwrap();
    let alice_untrusted = ServerId::parse("5e000000-0000-4000-8000-00000000a002").unwrap();
    let bob_shared = ServerId::parse("5e000000-0000-4000-8000-00000000b001").unwrap();

    let server = |id, owner, name, fp: &str, verified, revision| SeedServer {
        id,
        owner,
        display_name: name,
        hostname: SHARED_HOSTNAME,
        port: 22,
        username: "deploy",
        environment: Environment::Development,
        revision,
        enabled: true,
        approval_mode: ApprovalMode::default(),
        host_fingerprint: fp.to_owned(),
        verified,
    };
    world.seed_server(server(
        alice_shared,
        alice_id,
        "Alice shared host",
        ALICE_FINGERPRINT,
        true,
        2,
    ));
    world.seed_server(server(
        alice_untrusted,
        alice_id,
        "Alice untrusted host",
        "SHA256:AliceUntrustedFakeFakeFakeFakeFakeFakeF",
        false,
        1,
    ));
    world.seed_server(server(
        bob_shared,
        bob_id,
        "Bob shared host",
        BOB_FINGERPRINT,
        true,
        2,
    ));

    let alice_ssh_key = world.seed_credential(
        alice_shared,
        CredentialKind::SshPrivateKey,
        "FAKE-SECRET-alice-ssh-key",
    );
    world.seed_credential(
        alice_shared,
        CredentialKind::SudoPassword,
        "FAKE-SECRET-alice-sudo",
    );
    world.seed_credential(
        alice_untrusted,
        CredentialKind::SshPrivateKey,
        "FAKE-SECRET-alice-ssh-key-2",
    );
    let bob_ssh_key = world.seed_credential(
        bob_shared,
        CredentialKind::SshPrivateKey,
        "FAKE-SECRET-bob-ssh-key",
    );
    world.seed_credential(
        bob_shared,
        CredentialKind::SudoPassword,
        "FAKE-SECRET-bob-sudo",
    );

    Demo {
        world: world.clone(),
        alice: actor(world, alice_id),
        bob: actor(world, bob_id),
        servers: DemoServers {
            alice_shared,
            bob_shared,
            alice_untrusted,
        },
        alice_ssh_key,
        bob_ssh_key,
    }
}

/// Demo entry point: a fresh world with Alice and Bob seeded.
pub fn demo() -> Demo {
    let world = World::new(InstallationId::parse(INSTALLATION).unwrap());
    seed_demo(&world)
}

impl Demo {
    /// Resets the world and seeds it again. IDs and credentials are identical to before;
    /// previously issued tokens are gone, so use the returned demo.
    pub fn reset(&self) -> Demo {
        self.world.reset();
        seed_demo(&self.world)
    }

    /// Alice or Bob authenticated through the contract's own Identity trait.
    pub fn agent_ctx(&self, actor: &Actor) -> RequestContext {
        self.ctx_from_token(&actor.mcp_token)
    }

    pub fn ctx_from_token(&self, token: &str) -> RequestContext {
        let call = self.call_ctx();
        let services = self.world.services();
        let principal = super::memory::block_on(
            services
                .identity
                .validate_mcp_token(&call, BearerToken::new(token.to_owned())),
        )
        .expect("token valid");
        RequestContext::new(call, principal)
    }

    /// Interactive browser session context (can decide approvals).
    pub fn browser_ctx(&self, actor: &Actor) -> RequestContext {
        let call = self.call_ctx();
        let services = self.world.services();
        let principal = super::memory::block_on(services.identity.authenticate_browser(
            &call,
            BrowserCredentials {
                session_cookie: BearerToken::new(actor.session_cookie.clone()),
                csrf_token: Some(BearerToken::new(actor.csrf.clone())),
                unsafe_method: true,
            },
        ))
        .expect("session valid");
        RequestContext::new(call, principal)
    }

    pub fn call_ctx(&self) -> CallContext {
        CallContext {
            request_id: RequestId::parse("0b1c2d3e-4f50-4617-8899-aabbccddeeff").unwrap(),
            deadline: self.world.now().checked_add_millis(30_000).unwrap(),
        }
    }

    pub fn driver(&self) -> Driver {
        Driver {
            services: self.world.services(),
            world: self.world.clone(),
        }
    }
}

// ---------------------------------------------------------------------------------------
// Driver: the reference call sequence
// ---------------------------------------------------------------------------------------

/// Where a driven operation stopped.
#[derive(Debug)]
pub enum Driven {
    /// Completed, refused, failed-before-submission or unknown-after-submission.
    Finished(Box<OperationResult>),
    /// Human approval is required first; carries the stored approval.
    ApprovalRequired(Box<ApprovalSummary>),
}

impl Driven {
    pub fn finished(&self) -> Option<&OperationResult> {
        match self {
            Driven::Finished(r) => Some(r),
            Driven::ApprovalRequired(_) => None,
        }
    }
}

/// Mock risk classification by tool and arguments (stands in for Spec 03's classifier).
pub fn classify_risk(tool: &str, args: &Value) -> RiskLevel {
    if tool == "execute_command" {
        let command = args.get("command").and_then(Value::as_str).unwrap_or("");
        return if command.contains("rm -rf /") {
            RiskLevel::Critical
        } else if command.starts_with("rm ") {
            RiskLevel::High
        } else if ["id", "uname", "ls", "cat ", "whoami"]
            .iter()
            .any(|p| command == *p || command.starts_with(p))
        {
            RiskLevel::Low
        } else {
            RiskLevel::Medium
        };
    }
    legacy_tool(tool)
        .and_then(|t| t.risk)
        .unwrap_or(RiskLevel::Critical)
}

pub struct Driver {
    pub services: Services,
    world: World,
}

impl Driver {
    fn intent(
        &self,
        ctx: &RequestContext,
        request: &OperationRequest,
        target: &crate::backend::target::AuthorizedTarget,
    ) -> Result<OperationIntent, PublicError> {
        let now = self.world.now();
        let operation_id = self.world.next_operation_id();
        OperationIntent::new(IntentParts {
            operation_id,
            installation_id: ctx.principal.installation_id(),
            owner: ctx.principal.user_id(),
            server_id: target.server_id(),
            server_revision: target.revision(),
            tool: request.tool.clone(),
            tool_schema_version: request.tool_schema_version,
            arguments: request.arguments.clone(),
            risk: classify_risk(request.tool.as_str(), request.arguments.value()),
            created_at: now,
            expires_at: now.checked_add_millis(15 * 60 * 1000).unwrap(),
        })
        .map_err(|_| {
            PublicError::validation(
                ctx.request_id(),
                crate::public::errors::ValidationError::single(
                    "operation",
                    crate::public::errors::ViolationRule::InvalidFormat,
                ),
            )
        })
    }

    /// Resolve → intent → evaluate → refuse | approval request | submit (when allowed).
    pub async fn run(
        &self,
        ctx: &RequestContext,
        request: OperationRequest,
    ) -> ServiceResult<Driven> {
        let risk = classify_risk(request.tool.as_str(), request.arguments.value());
        let scope = if risk >= RiskLevel::Medium {
            Scope::ServersMutate
        } else {
            Scope::ServersInspect
        };
        let s = &self.services;
        // Ownership, enabled and trust are checked before any secret or network access.
        let target = s.targets.resolve(ctx, request.server_id, scope).await?;
        let intent = self.intent(ctx, &request, &target)?;
        let policy = s.executor.evaluate(ctx, &intent, &target).await?;
        match policy.verdict.clone() {
            PolicyVerdict::Denied { reason } => {
                let outcome = OperationOutcome::refused(reason, None);
                Ok(Driven::Finished(Box::new(self.refused(&intent, outcome))))
            }
            PolicyVerdict::RequiresApproval => Ok(Driven::ApprovalRequired(Box::new(
                s.approvals.request(ctx, intent, &policy).await?,
            ))),
            PolicyVerdict::Allowed => {
                let authority = ExecutionAuthority::without_approval(
                    &intent,
                    &target,
                    &policy,
                    self.world.now(),
                )
                .map_err(|_| PublicError::forbidden(ctx.request_id()))?;
                self.dispatch(ctx, intent, authority)
                    .await
                    .map(|r| Driven::Finished(Box::new(r)))
            }
        }
    }

    /// Executes a previously approved operation: re-resolve, re-evaluate, atomically consume,
    /// durably begin the audit record, submit.
    pub async fn execute_approved(
        &self,
        ctx: &RequestContext,
        approval_id: ApprovalId,
    ) -> ServiceResult<OperationResult> {
        let s = &self.services;
        let approval = s.approvals.get(ctx, approval_id).await?;
        let intent = approval.intent;
        let scope = if intent.risk() >= RiskLevel::Medium {
            Scope::ServersMutate
        } else {
            Scope::ServersInspect
        };
        let target = s.targets.resolve(ctx, intent.server_id(), scope).await?;
        let policy = s.executor.evaluate(ctx, &intent, &target).await?;
        let authority = s
            .approvals
            .consume(ctx, approval_id, &intent, &target, &policy)
            .await?;
        self.dispatch(ctx, intent, authority).await
    }

    async fn dispatch(
        &self,
        ctx: &RequestContext,
        intent: OperationIntent,
        authority: ExecutionAuthority,
    ) -> ServiceResult<OperationResult> {
        let s = &self.services;
        // If durable audit is unavailable the mutation is rejected and nothing is sent.
        let audit = s.audit.begin_operation(ctx, &intent, &authority).await?;
        s.executor
            .submit(
                ctx,
                SubmitRequest {
                    intent,
                    authority,
                    audit,
                },
            )
            .await
    }

    fn refused(&self, intent: &OperationIntent, outcome: OperationOutcome) -> OperationResult {
        let now = self.world.now();
        OperationResult {
            operation_id: intent.operation_id(),
            server_id: intent.server_id(),
            server_revision: intent.server_revision(),
            tool: intent.tool().clone(),
            submission: SubmissionState::NotSubmitted,
            outcome,
            approval_id: None,
            started_at: now,
            finished_at: now,
        }
    }

    /// Human decision helper (browser session + CSRF context required).
    pub async fn decide(
        &self,
        browser: &RequestContext,
        approval: &ApprovalSummary,
        decision: ApprovalDecision,
    ) -> ServiceResult<ApprovalSummary> {
        self.services
            .approvals
            .decide(
                browser,
                approval.approval_id,
                ApprovalDecisionRequest {
                    decision,
                    intent_digest: approval.intent_digest.clone(),
                },
            )
            .await
    }
}

/// Scripted failure scenarios by name, for demos.
pub fn script_timeout_before_submit(demo: &Demo, server: ServerId) {
    demo.world.script(server, ExecScript::TimeoutBeforeSubmit);
}

pub fn script_disconnect_after_submit(demo: &Demo, server: ServerId) {
    demo.world.script(server, ExecScript::DisconnectAfterSubmit);
}

/// Convenience for tests that need an agent principal without going through tokens.
pub fn principal_of(ctx: &RequestContext) -> &Principal {
    &ctx.principal
}
