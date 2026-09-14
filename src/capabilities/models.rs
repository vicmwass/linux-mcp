use std::collections::HashSet;

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