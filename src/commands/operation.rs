use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    // Read-only operations
    ReadOnly,

    // User management
    UserCreate,
    UserModify,
    UserDelete,

    // Group management
    GroupCreate,
    GroupModify,
    GroupDelete,

    // Package management
    PackageInstall,
    PackageRemove,
    PackageUpdate,

    // Service management
    ServiceStart,
    ServiceStop,
    ServiceRestart,
    ServiceEnable,
    ServiceDisable,

    // Filesystem
    FileRead,
    FileWrite,
    FileDelete,

    // System
    SystemReboot,
    SystemShutdown,

    // Networking
    NetworkRead,
    NetworkModify,

    // Unknown / unclassified
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl Operation {
    pub fn risk_level(&self) -> RiskLevel {
        match self {
            Operation::ReadOnly
            | Operation::NetworkRead
            | Operation::FileRead => RiskLevel::Low,

            Operation::UserCreate
            | Operation::UserModify
            | Operation::GroupCreate
            | Operation::GroupModify
            | Operation::PackageInstall
            | Operation::PackageUpdate
            | Operation::ServiceStart
            | Operation::ServiceRestart
            | Operation::ServiceEnable
            | Operation::ServiceDisable
            | Operation::NetworkModify
            | Operation::FileWrite => RiskLevel::Medium,

            Operation::UserDelete
            | Operation::GroupDelete
            | Operation::PackageRemove
            | Operation::ServiceStop
            | Operation::FileDelete => RiskLevel::High,

            Operation::SystemReboot
            | Operation::SystemShutdown
            | Operation::Unknown => RiskLevel::Critical,
        }
    }

    pub fn requires_sudo(&self) -> bool {
        match self {
            Operation::ReadOnly
            | Operation::NetworkRead
            | Operation::FileRead => false,

            _ => true,
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Operation::ReadOnly => "read_only",

            Operation::UserCreate => "user_create",
            Operation::UserModify => "user_modify",
            Operation::UserDelete => "user_delete",

            Operation::GroupCreate => "group_create",
            Operation::GroupModify => "group_modify",
            Operation::GroupDelete => "group_delete",

            Operation::PackageInstall => "package_install",
            Operation::PackageRemove => "package_remove",
            Operation::PackageUpdate => "package_update",

            Operation::ServiceStart => "service_start",
            Operation::ServiceStop => "service_stop",
            Operation::ServiceRestart => "service_restart",
            Operation::ServiceEnable => "service_enable",
            Operation::ServiceDisable => "service_disable",

            Operation::FileRead => "file_read",
            Operation::FileWrite => "file_write",
            Operation::FileDelete => "file_delete",

            Operation::SystemReboot => "system_reboot",
            Operation::SystemShutdown => "system_shutdown",

            Operation::NetworkRead => "network_read",
            Operation::NetworkModify => "network_modify",

            Operation::Unknown => "unknown",
        };

        write!(formatter, "{name}")
    }
}