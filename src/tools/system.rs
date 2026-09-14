use std::collections::HashSet;
use std::time::Duration;
use anyhow::Result;

use crate::ssh::SshManager;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitSystem {
    Systemd,
    OpenRc,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageManager {
    Apt,
    Dnf,
    Yum,
    Apk,
    Pacman,
    Zypper,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    pub distribution: String,
    pub version: String,
    pub kernel: String,
    pub architecture: String,
    pub init_system: InitSystem,
    pub package_manager: PackageManager,
    pub commands: HashSet<String>,
}

impl ServerCapabilities {
    pub fn supports_command(&self, command: &str) -> bool {
        self.commands.contains(command)
    }
}

pub async fn detect_capabilities(
    ssh_manager: &SshManager,
    server_id: &str,
) -> Result<ServerCapabilities> {
    let command = r#"printf '%s\n' \
    "=== OS ===" \
    "$(cat /etc/os-release 2>/dev/null)" \
    "=== KERNEL ===" \
    "$(uname -srm)" \
    "=== ARCHITECTURE ===" \
    "$(uname -m)" \
    "=== INIT ===" \
    "$(if command -v systemctl >/dev/null 2>&1; then echo systemd; elif command -v openrc >/dev/null 2>&1; then echo openrc; else echo unknown; fi)" \
    "=== PACKAGE_MANAGER ===" \
    "$(if command -v apt-get >/dev/null 2>&1; then echo apt; elif command -v dnf >/dev/null 2>&1; then echo dnf; elif command -v yum >/dev/null 2>&1; then echo yum; elif command -v apk >/dev/null 2>&1; then echo apk; elif command -v pacman >/dev/null 2>&1; then echo pacman; elif command -v zypper >/dev/null 2>&1; then echo zypper; else echo unknown; fi)" \
    "=== COMMANDS ===" \
    "$(for cmd in useradd usermod userdel groupadd groupdel passwd chage systemctl service journalctl apt apt-get dnf yum rpm dpkg apk pacman zypper ip ss ping curl wget tar gzip unzip rsync; do if command -v "$cmd" >/dev/null 2>&1; then echo "$cmd"; fi; done)"
    "#;

    let result = ssh_manager
        .execute(server_id, command, Duration::from_secs(30) )
        .await?;

    if result.exit_code != 0 {
        anyhow::bail!(
            "Failed to detect server capabilities: {}",
            result.stderr.trim()
        );
    }

    parse_capabilities(&result.stdout)
}

fn parse_capabilities(output: &str) -> Result<ServerCapabilities> {
    let mut distribution = String::from("Unknown");
    let mut version = String::from("Unknown");
    let mut kernel = String::from("Unknown");
    let mut architecture = String::from("Unknown");
    let mut init_system = InitSystem::Unknown;
    let mut package_manager = PackageManager::Unknown;

    let mut commands = HashSet::new();

    let mut section = "";

    for line in output.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        match line {
            "=== OS ===" => {
                section = "os";
                continue;
            }

            "=== KERNEL ===" => {
                section = "kernel";
                continue;
            }

            "=== ARCHITECTURE ===" => {
                section = "architecture";
                continue;
            }

            "=== INIT ===" => {
                section = "init";
                continue;
            }

            "=== PACKAGE_MANAGER ===" => {
                section = "package_manager";
                continue;
            }

            "=== COMMANDS ===" => {
                section = "commands";
                continue;
            }

            _ => {}
        }

        match section {
            "os" => {
                if let Some(value) = line.strip_prefix("NAME=") {
                    distribution = clean_os_value(value);
                }

                if let Some(value) = line.strip_prefix("VERSION_ID=") {
                    version = clean_os_value(value);
                }
            }

            "kernel" => {
                kernel = line.to_string();
            }

            "architecture" => {
                architecture = line.to_string();
            }

            "init" => {
                init_system = match line {
                    "systemd" => InitSystem::Systemd,
                    "openrc" => InitSystem::OpenRc,
                    _ => InitSystem::Unknown,
                };
            }

            "package_manager" => {
                package_manager = match line {
                    "apt" => PackageManager::Apt,
                    "dnf" => PackageManager::Dnf,
                    "yum" => PackageManager::Yum,
                    "apk" => PackageManager::Apk,
                    "pacman" => PackageManager::Pacman,
                    "zypper" => PackageManager::Zypper,
                    _ => PackageManager::Unknown,
                };
            }

            "commands" => {
                commands.insert(line.to_string());
            }

            _ => {}
        }
    }

    Ok(ServerCapabilities {
        distribution,
        version,
        kernel,
        architecture,
        init_system,
        package_manager,
        commands,
    })
}

fn clean_os_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .to_string()
}