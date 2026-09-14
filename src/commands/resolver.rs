use anyhow::{bail, Result};

use crate::capabilities::{
    PackageManager,
    ServerCapabilities,
};

use super::Operation;

pub struct ResolvedCommand {
    pub operation: Operation,
    pub command: String,
}

pub fn resolve(
    command: &str,
    capabilities: &ServerCapabilities,
) -> Result<ResolvedCommand> {
    let parsed = super::parser::parse(command)?;

    validate_operation(&parsed.operation, capabilities, command)?;

    Ok(ResolvedCommand {
        operation: parsed.operation,
        command: parsed.command,
    })
}

fn validate_operation(
    operation: &Operation,
    capabilities: &ServerCapabilities,
    command: &str,
) -> Result<()> {
    match operation {
        Operation::PackageInstall
        | Operation::PackageRemove
        | Operation::PackageUpdate => {
            validate_package_command(capabilities, command)?;
        }

        Operation::ServiceStart
        | Operation::ServiceStop
        | Operation::ServiceRestart
        | Operation::ServiceEnable
        | Operation::ServiceDisable => {
            validate_service_command(capabilities)?;
        }

        Operation::UserCreate
        | Operation::UserModify
        | Operation::UserDelete => {
            validate_command_exists(
                capabilities,
                match operation {
                    Operation::UserCreate => "useradd",
                    Operation::UserModify => "usermod",
                    Operation::UserDelete => "userdel",
                    _ => unreachable!(),
                },
            )?;
        }

        Operation::GroupCreate
        | Operation::GroupModify
        | Operation::GroupDelete => {
            validate_command_exists(
                capabilities,
                match operation {
                    Operation::GroupCreate => "groupadd",
                    Operation::GroupModify => "groupmod",
                    Operation::GroupDelete => "groupdel",
                    _ => unreachable!(),
                },
            )?;
        }

        Operation::Unknown => {
            bail!("Unable to determine a supported Linux operation");
        }

        _ => {}
    }

    Ok(())
}

fn validate_package_command(
    capabilities: &ServerCapabilities,
    command: &str,
) -> Result<()> {
    let expected_manager = match capabilities.package_manager {
        PackageManager::Apt => "apt",
        PackageManager::Dnf => "dnf",
        PackageManager::Yum => "yum",
        PackageManager::Apk => "apk",
        PackageManager::Pacman => "pacman",
        PackageManager::Zypper => "zypper",
        PackageManager::Unknown => {
            bail!("No supported package manager detected");
        }
    };

    let actual_manager = command
        .strip_prefix("sudo ")
        .unwrap_or(command)
        .split_whitespace()
        .next()
        .unwrap_or("");

    if actual_manager != expected_manager {
        bail!(
            "Package manager mismatch: server uses '{}', \
             but command uses '{}'",
            expected_manager,
            actual_manager
        );
    }

    validate_command_exists(capabilities, expected_manager)
}

fn validate_service_command(
    capabilities: &ServerCapabilities,
) -> Result<()> {
    match capabilities.init_system {
        crate::capabilities::InitSystem::Systemd => {
            validate_command_exists(capabilities, "systemctl")
        }

        crate::capabilities::InitSystem::OpenRc => {
            validate_command_exists(capabilities, "service")
        }

        crate::capabilities::InitSystem::Unknown => {
            bail!("Unable to determine the server's init system");
        }
    }
}

fn validate_command_exists(
    capabilities: &ServerCapabilities,
    command: &str,
) -> Result<()> {
    if !capabilities.supports_command(command) {
        bail!(
            "Required command '{}' is not available on the target server",
            command
        );
    }

    Ok(())
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

    fn ubuntu_capabilities() -> ServerCapabilities {
        ServerCapabilities {
            distribution: "Ubuntu".to_string(),
            version: "24.04".to_string(),
            kernel: "6.8".to_string(),
            architecture: "x86_64".to_string(),
            init_system: InitSystem::Systemd,
            package_manager: PackageManager::Apt,
            commands: HashSet::from([
                "apt".to_string(),
                "useradd".to_string(),
                "usermod".to_string(),
                "userdel".to_string(),
                "groupadd".to_string(),
                "groupmod".to_string(),
                "groupdel".to_string(),
                "systemctl".to_string(),
            ]),
        }
    }

    fn rhel_capabilities() -> ServerCapabilities {
        ServerCapabilities {
            distribution: "RHEL".to_string(),
            version: "9".to_string(),
            kernel: "5.14".to_string(),
            architecture: "x86_64".to_string(),
            init_system: InitSystem::Systemd,
            package_manager: PackageManager::Dnf,
            commands: HashSet::from([
                "dnf".to_string(),
                "useradd".to_string(),
                "usermod".to_string(),
                "userdel".to_string(),
                "groupadd".to_string(),
                "groupmod".to_string(),
                "groupdel".to_string(),
                "systemctl".to_string(),
            ]),
        }
    }

    #[test]
    fn ubuntu_accepts_apt() {
        let capabilities = ubuntu_capabilities();

        let result =
            resolve("sudo apt install nginx", &capabilities);

        assert!(result.is_ok());
    }

    #[test]
    fn ubuntu_rejects_dnf() {
        let capabilities = ubuntu_capabilities();

        let result =
            resolve("sudo dnf install nginx", &capabilities);

        assert!(result.is_err());
    }

    #[test]
    fn rhel_accepts_dnf() {
        let capabilities = rhel_capabilities();

        let result =
            resolve("sudo dnf install nginx", &capabilities);

        assert!(result.is_ok());
    }

    #[test]
    fn rhel_rejects_apt() {
        let capabilities = rhel_capabilities();

        let result =
            resolve("sudo apt install nginx", &capabilities);

        assert!(result.is_err());
    }

    #[test]
    fn user_creation_requires_useradd() {
        let capabilities = ubuntu_capabilities();

        let result =
            resolve("sudo useradd john", &capabilities);

        assert!(result.is_ok());
    }

    #[test]
    fn group_modification_requires_groupmod() {
        let capabilities = ubuntu_capabilities();

        let result = resolve(
            "sudo groupmod -n admins developers",
            &capabilities,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn network_read_commands_resolve() {
        let capabilities = ubuntu_capabilities();

        let result =
            resolve("curl https://example.com", &capabilities);

        assert!(result.is_ok());
    }
}