use anyhow::{bail, Result};
use std::time::Duration;
use crate::ssh::SshManager;

use super::models::{
    InitSystem,
    PackageManager,
    ServerCapabilities,
};

pub async fn detect(
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
"$(for cmd in useradd usermod userdel groupadd groupmod groupdel passwd chage systemctl service journalctl apt apt-get dnf yum rpm dpkg apk pacman zypper ip ss ping curl wget tar gzip unzip rsync bash sh python3 python perl ruby node; do if command -v "$cmd" >/dev/null 2>&1; then echo "$cmd"; fi; done)"
"#;

    let result = ssh_manager
        .execute(server_id, command,  Duration::from_secs(30))
        .await?;

    if result.exit_code != 0 {
        bail!(
            "Failed to detect server capabilities: {}",
            result.stderr.trim()
        );
    }

    parse(&result.stdout)
}

fn parse(output: &str) -> Result<ServerCapabilities> {
    let os = section(output, "=== OS ===");
    let kernel = section(output, "=== KERNEL ===");
    let architecture = section(output, "=== ARCHITECTURE ===");
    let init = section(output, "=== INIT ===");
    let package_manager = section(output, "=== PACKAGE_MANAGER ===");
    let commands = section_lines(output, "=== COMMANDS ===");

    let distribution = os
        .lines()
        .find_map(|line| line.strip_prefix("ID="))
        .map(clean_value)
        .unwrap_or_else(|| "unknown".to_string());

    let version = os
        .lines()
        .find_map(|line| line.strip_prefix("VERSION_ID="))
        .map(clean_value)
        .unwrap_or_else(|| "unknown".to_string());

    let init_system = match init.trim() {
        "systemd" => InitSystem::Systemd,
        "openrc" => InitSystem::OpenRc,
        _ => InitSystem::Unknown,
    };

    let package_manager = match package_manager.trim() {
        "apt" => PackageManager::Apt,
        "dnf" => PackageManager::Dnf,
        "yum" => PackageManager::Yum,
        "apk" => PackageManager::Apk,
        "pacman" => PackageManager::Pacman,
        "zypper" => PackageManager::Zypper,
        _ => PackageManager::Unknown,
    };

    Ok(ServerCapabilities {
        distribution,
        version,
        kernel: kernel.trim().to_string(),
        architecture: architecture.trim().to_string(),
        init_system,
        package_manager,
        commands,
    })
}

fn section(output: &str, header: &str) -> String {
    let mut collecting = false;
    let mut lines = Vec::new();

    for line in output.lines() {
        if line == header {
            collecting = true;
            continue;
        }

        if collecting && line.starts_with("===") {
            break;
        }

        if collecting {
            lines.push(line);
        }
    }

    lines.join("\n")
}

fn section_lines(
    output: &str,
    header: &str,
) -> std::collections::HashSet<String> {
    section(output, header)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn clean_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}