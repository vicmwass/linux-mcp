use anyhow::{bail, Result};

use crate::capabilities::ServerCapabilities;
use crate::commands::{Operation, RiskLevel};
use crate::execution::ExecutionAnalysis;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allowed,
    RequiresConfirmation,
    Denied,
}

#[derive(Debug, Clone)]
pub struct SecurityPolicy;

impl SecurityPolicy {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        analysis: &ExecutionAnalysis,
        capabilities: &ServerCapabilities,
        confirmed: bool,
        command: &str,
        is_root: bool,
    ) -> Result<PolicyDecision> {
        // Catastrophic deletes are denied outright; confirmation cannot override them.
        if is_catastrophic_delete(command) {
            return Ok(PolicyDecision::Denied);
        }

        /*
         * Every operation in the execution must be understood
         * and explicitly allowed.
         */
        for operation in &analysis.operations {
            if matches!(operation, Operation::Unknown) {
                return Ok(PolicyDecision::Denied);
            }

            if !self.operation_allowed(operation) {
                return Ok(PolicyDecision::Denied);
            }
        }

        /*
         * Make sure the target server supports everything
         * required by the execution.
         */
        self.validate_capabilities(
            &analysis.operations,
            capabilities,
        )?;

        /*
         * Critical operations are never allowed through
         * the normal command execution interface.
         */
        if matches!(analysis.risk, RiskLevel::Critical) {
            return Ok(PolicyDecision::Denied);
        }

        // Sudo commands execute as root, so every privileged operation needs explicit confirmation.
        if is_root && analysis.requires_sudo && !confirmed {
            return Ok(PolicyDecision::RequiresConfirmation);
        }

        /*
         * High-risk operations require explicit confirmation.
         */
        if matches!(analysis.risk, RiskLevel::High) {
            if !confirmed {
                return Ok(
                    PolicyDecision::RequiresConfirmation
                );
            }
        }

        /*
         * Network operations can be handled more strictly
         * later. For now, they are allowed if the operation
         * itself passes policy.
         */
        Ok(PolicyDecision::Allowed)
    }

    fn operation_allowed(
        &self,
        operation: &Operation,
    ) -> bool {
        matches!(
            operation,
            Operation::ReadOnly
                | Operation::NetworkRead
                | Operation::FileRead
                | Operation::UserCreate
                | Operation::UserModify
                | Operation::UserDelete
                | Operation::GroupCreate
                | Operation::GroupModify
                | Operation::GroupDelete
                | Operation::PackageInstall
                | Operation::PackageRemove
                | Operation::PackageUpdate
                | Operation::ServiceStart
                | Operation::ServiceStop
                | Operation::ServiceRestart
                | Operation::ServiceEnable
                | Operation::ServiceDisable
                | Operation::FileWrite
                | Operation::FileDelete
        )
    }

    fn validate_capabilities(
        &self,
        operations: &[Operation],
        capabilities: &ServerCapabilities,
    ) -> Result<()> {
        for operation in operations {
            match operation {
                Operation::PackageInstall
                | Operation::PackageRemove
                | Operation::PackageUpdate => {
                    if matches!(
                        capabilities.package_manager,
                        crate::capabilities::PackageManager::Unknown
                    ) {
                        bail!(
                            "No supported package manager detected"
                        );
                    }
                }

                Operation::ServiceStart
                | Operation::ServiceStop
                | Operation::ServiceRestart
                | Operation::ServiceEnable
                | Operation::ServiceDisable => {
                    if matches!(
                        capabilities.init_system,
                        crate::capabilities::InitSystem::Unknown
                    ) {
                        bail!(
                            "No supported init system detected"
                        );
                    }
                }

                Operation::UserCreate => {
                    self.require_command(
                        capabilities,
                        "useradd",
                    )?;
                }

                Operation::UserModify => {
                    self.require_command(
                        capabilities,
                        "usermod",
                    )?;
                }

                Operation::UserDelete => {
                    self.require_command(
                        capabilities,
                        "userdel",
                    )?;
                }

                Operation::GroupCreate => {
                    self.require_command(
                        capabilities,
                        "groupadd",
                    )?;
                }

                Operation::GroupModify => {
                    self.require_command(
                        capabilities,
                        "groupmod",
                    )?;
                }

                Operation::GroupDelete => {
                    self.require_command(
                        capabilities,
                        "groupdel",
                    )?;
                }

                _ => {}
            }
        }

        Ok(())
    }

    fn require_command(
        &self,
        capabilities: &ServerCapabilities,
        command: &str,
    ) -> Result<()> {
        if !capabilities.supports_command(command) {
            bail!(
                "Required command '{}' is not available \
                 on the target server",
                command
            );
        }

        Ok(())
    }
}

// Detects rm/rmdir invocations that recursively/forcefully target root-level or wildcard paths.
fn is_catastrophic_delete(command: &str) -> bool {
    const DANGEROUS_TARGETS: &[&str] = &[
        "/", "/*", ".", "..", "~", "*", "/root", "/root/*", "/home", "/home/*",
        "/etc", "/etc/*", "/boot", "/boot/*", "/var", "/var/*", "/usr", "/usr/*",
        "/bin", "/bin/*", "/sbin", "/sbin/*", "/lib", "/lib/*", "/opt", "/opt/*",
        "/dev", "/dev/*", "/proc", "/proc/*", "/sys", "/sys/*",
    ];

    let normalized = command
        .trim()
        .strip_prefix("sudo ")
        .unwrap_or(command.trim())
        .trim();

    let tokens: Vec<&str> = normalized.split_whitespace().collect();

    let Some((executable, arguments)) = tokens.split_first() else {
        return false;
    };

    if *executable != "rm" && *executable != "rmdir" {
        return false;
    }

    let mut recursive = *executable == "rmdir";
    let mut force = *executable == "rmdir";

    for argument in arguments {
        if let Some(short_flags) = argument.strip_prefix('-') {
            if !short_flags.starts_with('-') {
                if short_flags.contains('r') || short_flags.contains('R') {
                    recursive = true;
                }

                if short_flags.contains('f') {
                    force = true;
                }

                continue;
            }

            match *argument {
                "--recursive" => recursive = true,
                "--force" => force = true,
                _ => {}
            }
        }
    }

    if !recursive || !force {
        return false;
    }

    arguments
        .iter()
        .filter(|argument| !argument.starts_with('-'))
        .any(|argument| {
            let trimmed = argument.trim_end_matches('/');
            let trimmed = if trimmed.is_empty() { "/" } else { trimmed };
            DANGEROUS_TARGETS.contains(&trimmed)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::{
        InitSystem,
        PackageManager,
        ServerCapabilities,
    };
    use std::collections::HashSet;

    fn capabilities() -> ServerCapabilities {
        ServerCapabilities {
            distribution: "ubuntu".to_string(),
            version: "24.04".to_string(),
            kernel: "6.8".to_string(),
            architecture: "x86_64".to_string(),
            init_system: InitSystem::Systemd,
            package_manager: PackageManager::Apt,
            commands: [
                "useradd",
                "usermod",
                "userdel",
                "groupadd",
                "groupmod",
                "groupdel",
            ]
            .into_iter()
            .map(String::from)
            .collect::<HashSet<_>>(),
        }
    }

    #[test]
    fn allows_read_only_operation() {
        let policy = SecurityPolicy::new();

        let mut analysis = ExecutionAnalysis::new();
        analysis.add_operation(Operation::ReadOnly);

        let decision = policy
            .evaluate(
                &analysis,
                &capabilities(),
                false,
                "cat /etc/os-release",
                false,
            )
            .unwrap();

        assert_eq!(
            decision,
            PolicyDecision::Allowed
        );
    }

    #[test]
    fn requires_confirmation_for_high_risk() {
        let policy = SecurityPolicy::new();

        let mut analysis = ExecutionAnalysis::new();
        analysis.add_operation(
            Operation::FileDelete
        );

        let decision = policy
            .evaluate(
                &analysis,
                &capabilities(),
                false,
                "rm /tmp/report.log",
                false,
            )
            .unwrap();

        assert_eq!(
            decision,
            PolicyDecision::RequiresConfirmation
        );
    }

    #[test]
    fn allows_confirmed_high_risk_operation() {
        let policy = SecurityPolicy::new();

        let mut analysis = ExecutionAnalysis::new();
        analysis.add_operation(
            Operation::FileDelete
        );

        let decision = policy
            .evaluate(
                &analysis,
                &capabilities(),
                true,
                "rm /tmp/report.log",
                false,
            )
            .unwrap();

        assert_eq!(
            decision,
            PolicyDecision::Allowed
        );
    }

    #[test]
    fn denies_unknown_operation() {
        let policy = SecurityPolicy::new();

        let mut analysis = ExecutionAnalysis::new();
        analysis.add_operation(
            Operation::Unknown
        );

        let decision = policy
            .evaluate(
                &analysis,
                &capabilities(),
                true,
                "some-unknown-tool --flag",
                false,
            )
            .unwrap();

        assert_eq!(
            decision,
            PolicyDecision::Denied
        );
    }

    #[test]
    fn compound_operation_uses_highest_risk() {
        let policy = SecurityPolicy::new();

        let mut analysis = ExecutionAnalysis::new();

        analysis.add_operation(
            Operation::ReadOnly
        );

        analysis.add_operation(
            Operation::ServiceRestart
        );

        analysis.add_operation(
            Operation::FileDelete
        );

        assert_eq!(
            analysis.risk,
            RiskLevel::High
        );

        let decision = policy
            .evaluate(
                &analysis,
                &capabilities(),
                false,
                "sudo rm /var/log/app.log",
                false,
            )
            .unwrap();

        assert_eq!(
            decision,
            PolicyDecision::RequiresConfirmation
        );
    }

#[test]
fn denies_critical_operation() {
    let policy = SecurityPolicy::new();

    let mut analysis = ExecutionAnalysis::new();

    analysis.add_operation(
        Operation::SystemShutdown
    );

    let decision = policy
        .evaluate(
            &analysis,
            &capabilities(),
            true,
            "sudo shutdown -h now",
            false,
        )
        .unwrap();

    assert_eq!(
        decision,
        PolicyDecision::Denied
    );
}

#[test]
fn requires_confirmation_for_root_privileged_medium_risk() {
    let policy = SecurityPolicy::new();

    let mut analysis = ExecutionAnalysis::new();
    analysis.add_operation(Operation::UserModify);

    let decision = policy
        .evaluate(
            &analysis,
            &capabilities(),
            false,
            "usermod -aG sudo alice",
            true,
        )
        .unwrap();

    assert_eq!(decision, PolicyDecision::RequiresConfirmation);
}

#[test]
fn allows_confirmed_root_privileged_medium_risk() {
    let policy = SecurityPolicy::new();

    let mut analysis = ExecutionAnalysis::new();
    analysis.add_operation(Operation::UserModify);

    let decision = policy
        .evaluate(
            &analysis,
            &capabilities(),
            true,
            "usermod -aG sudo alice",
            true,
        )
        .unwrap();

    assert_eq!(decision, PolicyDecision::Allowed);
}

#[test]
fn denies_catastrophic_delete_even_when_confirmed() {
    let policy = SecurityPolicy::new();

    let mut analysis = ExecutionAnalysis::new();
    analysis.add_operation(Operation::FileDelete);

    let decision = policy
        .evaluate(
            &analysis,
            &capabilities(),
            true,
            "sudo rm -rf /",
            true,
        )
        .unwrap();

    assert_eq!(decision, PolicyDecision::Denied);
}

#[test]
fn denies_catastrophic_wildcard_delete() {
    let policy = SecurityPolicy::new();

    let mut analysis = ExecutionAnalysis::new();
    analysis.add_operation(Operation::FileDelete);

    let decision = policy
        .evaluate(
            &analysis,
            &capabilities(),
            true,
            "rm -rf /*",
            false,
        )
        .unwrap();

    assert_eq!(decision, PolicyDecision::Denied);
}

#[test]
fn allows_non_catastrophic_recursive_delete() {
    let policy = SecurityPolicy::new();

    let mut analysis = ExecutionAnalysis::new();
    analysis.add_operation(Operation::FileDelete);

    let decision = policy
        .evaluate(
            &analysis,
            &capabilities(),
            true,
            "rm -rf /opt/myapp/build",
            false,
        )
        .unwrap();

    assert_eq!(decision, PolicyDecision::Allowed);
}
}