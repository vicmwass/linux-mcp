use crate::capabilities::manager::CapabilityManager;
use crate::execution::ExecutionEngine;
use std::sync::Arc;

use crate::security::PolicyDecision;
use crate::tools::{system, users};
use rmcp::{
    ServerHandler, handler::server::wrapper::Parameters, schemars, tool, tool_handler, tool_router,
};
use tokio::sync::Mutex;

use crate::{security::SecurityPolicy, ssh::SshManager};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecuteCommandParams {
    /// ID of the configured Linux server.
    pub server_id: String,

    /// Linux command to execute.
    pub command: String,

    /// Explicit confirmation for high-risk operations.
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InspectCommandParams {
    /// ID of the configured Linux server.
    pub server_id: String,
    /// Executable name or absolute path. Shell syntax is not accepted.
    pub executable: String,
    /// Positional arguments passed as individual argv entries.
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Clone)]
pub struct LinuxMcpServer {
    ssh_manager: Arc<SshManager>,
    capability_manager: Arc<CapabilityManager>,
    security_policy: SecurityPolicy,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListUsersParams {
    /// ID of the configured Linux server.
    pub server_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetUserParams {
    /// ID of the configured Linux server.
    pub server_id: String,

    /// Username to retrieve.
    pub username: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateUserParams {
    /// ID of the configured Linux server.
    pub server_id: String,

    /// Username to retrieve.
    pub username: String,

    /// Optional home directory.
    pub home: Option<String>,

    /// Optional login shell, e.g. /bin/bash.
    pub shell: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DisableUserParams {
    /// ID of the configured Linux server.
    pub server_id: String,

    /// Username to retrieve.
    pub username: String,
}
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EnableUserParams {
    /// ID of the configured Linux server.
    pub server_id: String,

    /// Username to enable.
    pub username: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteUserParams {
    /// ID of the configured Linux server.
    pub server_id: String,

    /// Username to delete.
    pub username: String,

    /// Must be true to permanently delete the account.
    pub confirm: bool,

    /// Also remove the user's home directory.
    pub remove_home: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetServerCapabilitiesParams {
    /// ID of the configured Linux server.
    pub server_id: String,
}
#[tool_router]
impl LinuxMcpServer {
    pub fn new(ssh_manager: SshManager) -> Self {
        let ssh_manager = Arc::new(ssh_manager);

        Self {
            capability_manager: Arc::new(CapabilityManager::new(ssh_manager.clone())),
            ssh_manager,
            security_policy: SecurityPolicy::new(),
        }
    }

    #[tool(
        name = "execute_command",
        description = "Execute a Linux command on a configured server after capability, operation, and security-policy validation."
    )]
    async fn execute_command(
        &self,
        Parameters(params): Parameters<ExecuteCommandParams>,
    ) -> String {
        // 1. Detect the target server's capabilities.
        let capabilities = match self.capability_manager.get(&params.server_id).await {
            Ok(capabilities) => capabilities,

            Err(error) => {
                return format!("Failed to detect server capabilities:\n{error:#}");
            }
        };

        // 2. Parse and resolve the requested command.
        let resolved = match crate::commands::resolve(&params.command, &capabilities) {
            Ok(resolved) => resolved,

            Err(error) => {
                return format!("Command rejected:\n{error:#}");
            }
        };

        let analysis = crate::execution::analyze(resolved.operation.clone());

        // A sudo command connects and executes as root, so it always requires confirmation.
        let is_root = crate::commands::is_sudo_command(&resolved.command);

        // 3. Apply the security policy.
        let decision = match self.security_policy.evaluate(
            &analysis,
            &capabilities,
            params.confirm,
            &resolved.command,
            is_root,
        ) {
            Ok(decision) => decision,

            Err(error) => {
                return format!("Security policy rejected the command:\n{error:#}");
            }
        };

        // 4. Handle the policy decision.
        match decision {
            PolicyDecision::Allowed => {}

            PolicyDecision::RequiresConfirmation => {
                return "This operation requires explicit confirmation.".into();
            }

            PolicyDecision::Denied => {
                return "This operation is not permitted.".into();
            }
        }

        // 5. Execute only after all validation succeeds.
        let plan = crate::execution::ExecutionPlan::command(
            params.server_id.clone(),
            resolved.command.clone(),
            analysis,
            params.confirm,
        );
        let engine = crate::execution::ExecutionEngine::new(&self.ssh_manager);

        match engine.execute(&plan).await {
            Ok(result) => {
                format!(
                    "Exit code: {}\n\
                 STDOUT:\n{}\n\
                 STDERR:\n{}",
                    result.exit_code, result.stdout, result.stderr,
                )
            }

            Err(error) => {
                format!("Command execution failed:\n{error:#}")
            }
        }
    }

    #[tool(
        name = "inspect_command",
        description = "Run a read-only diagnostic command with structured arguments as the configured SSH user. No shell syntax, sudo, or command chaining is permitted."
    )]
    async fn inspect_command(
        &self,
        Parameters(params): Parameters<InspectCommandParams>,
    ) -> String {
        if let Err(error) = validate_inspection_input(&params.executable, &params.args) {
            return format!("Inspection rejected:\n{error:#}");
        }

        let command = std::iter::once(params.executable.as_str())
            .chain(params.args.iter().map(String::as_str))
            .map(shell_quote)
            .collect::<Vec<_>>()
            .join(" ");

        let engine = crate::execution::ExecutionEngine::new(&self.ssh_manager);
        match engine.execute_inspection(&params.server_id, &command).await {
            Ok(result) => format!(
                "Exit code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                result.exit_code, result.stdout, result.stderr
            ),
            Err(error) => format!("Inspection failed:\n{error:#}"),
        }
    }
    #[tool(
        name = "list_servers",
        description = "List all Linux servers configured in the MCP server."
    )]
    async fn list_servers(&self) -> String {
        let servers = self.ssh_manager.list_servers();

        if servers.is_empty() {
            return "No Linux servers are configured.".to_string();
        }

        servers
            .iter()
            .map(|server| {
                format!(
                    "ID: {}\nHost: {}\nPort: {}\nUser: {}\nEnvironment: {}",
                    server.id, server.hostname, server.port, server.username, server.environment
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[tool(
        name = "list_users",
        description = "List users configured on a Linux server."
    )]
    async fn list_users(&self, Parameters(params): Parameters<ListUsersParams>) -> String {
        match users::list_users(&self.ssh_manager, &params.server_id).await {
            Ok(users) => users,
            Err(error) => {
                format!("Failed to list users:\n{error:#}")
            }
        }
    }

    #[tool(
        name = "get_user",
        description = "Get detailed information about a user on a Linux server."
    )]
    async fn get_user(&self, Parameters(params): Parameters<GetUserParams>) -> String {
        match users::get_user(&self.ssh_manager, &params.server_id, &params.username).await {
            Ok(user) => user,

            Err(error) => {
                format!("Failed to get user:\n{error:#}")
            }
        }
    }

    #[tool(
        name = "create_user",
        description = "Create a Linux user account on a configured server."
    )]
    async fn create_user(&self, Parameters(params): Parameters<CreateUserParams>) -> String {
        match users::create_user(
            &self.ssh_manager,
            &params.server_id,
            &params.username.to_lowercase(),
            params.home.as_deref(),
            params.shell.as_deref(),
        )
        .await
        {
            Ok(message) => message,
            Err(error) => format!("Failed to create user:\n{error:#}"),
        }
    }

    #[tool(
        name = "disable_user",
        description = "Disable a Linux user account without deleting it."
    )]
    async fn disable_user(&self, Parameters(params): Parameters<DisableUserParams>) -> String {
        match users::disable_user(&self.ssh_manager, &params.server_id, &params.username).await {
            Ok(message) => message,
            Err(error) => format!("Failed to disable user:\n{error:#}"),
        }
    }
    #[tool(
        name = "enable_user",
        description = "Enable a previously disabled Linux user account."
    )]
    async fn enable_user(&self, Parameters(params): Parameters<EnableUserParams>) -> String {
        match users::enable_user(&self.ssh_manager, &params.server_id, &params.username).await {
            Ok(message) => message,
            Err(error) => format!("Failed to enable user:\n{error:#}"),
        }
    }

    #[tool(
        name = "delete_user",
        description = "Permanently delete a Linux user account. Requires explicit confirmation."
    )]
    async fn delete_user(&self, Parameters(params): Parameters<DeleteUserParams>) -> String {
        match users::delete_user(
            &self.ssh_manager,
            &params.server_id,
            &params.username,
            params.confirm,
            params.remove_home,
        )
        .await
        {
            Ok(message) => message,
            Err(error) => format!("Failed to delete user:\n{error:#}"),
        }
    }

    #[tool(
        name = "get_server_capabilities",
        description = "Detect the operating system, distribution, package manager, init system, architecture, and available Linux commands on a server."
    )]
    async fn get_server_capabilities(
        &self,
        Parameters(params): Parameters<GetServerCapabilitiesParams>,
    ) -> String {
        match system::detect_capabilities(&self.ssh_manager, &params.server_id).await {
            Ok(capabilities) => {
                let mut commands: Vec<_> = capabilities.commands.iter().cloned().collect();

                commands.sort();

                format!(
                    "Server: {}\n\
                 Distribution: {}\n\
                 Version: {}\n\
                 Kernel: {}\n\
                 Architecture: {}\n\
                 Init system: {:?}\n\
                 Package manager: {:?}\n\
                 Available commands:\n{}",
                    params.server_id,
                    capabilities.distribution,
                    capabilities.version,
                    capabilities.kernel,
                    capabilities.architecture,
                    capabilities.init_system,
                    capabilities.package_manager,
                    commands
                        .iter()
                        .map(|command| format!("  {command}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }

            Err(error) => {
                format!("Failed to detect server capabilities:\n{error:#}")
            }
        }
    }
}

fn validate_inspection_input(executable: &str, args: &[String]) -> anyhow::Result<()> {
    anyhow::ensure!(!executable.is_empty(), "Executable cannot be empty");
    anyhow::ensure!(executable.len() <= 256, "Executable is too long");
    anyhow::ensure!(
        executable.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/')),
        "Executable contains invalid characters"
    );
    anyhow::ensure!(!executable.contains(".."), "Path traversal is not permitted");
    anyhow::ensure!(executable != "sudo" && executable != "sh" && executable != "bash" && executable != "dash" && executable != "zsh", "Shell and sudo executables are not permitted");
    anyhow::ensure!(args.len() <= 32, "Too many arguments");
    for arg in args {
        anyhow::ensure!(arg.len() <= 4096, "Argument is too long");
        anyhow::ensure!(!arg.contains('\0') && !arg.contains('\n') && !arg.contains('\r'), "Arguments cannot contain NUL or newlines");
    }
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[tool_handler]
impl ServerHandler for LinuxMcpServer {}
