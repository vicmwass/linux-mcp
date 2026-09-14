use anyhow::{bail, Result};

use super::Operation;

pub struct ParsedCommand {
    pub operation: Operation,
    pub command: String,
}

pub fn parse(command: &str) -> Result<ParsedCommand> {
    let command = command.trim();

    if command.is_empty() {
        bail!("Command cannot be empty");
    }

    reject_shell_constructs(command)?;

    let normalized = command
        .strip_prefix("sudo ")
        .unwrap_or(command)
        .trim();

    let executable = normalized
        .split_whitespace()
        .next()
        .unwrap_or("");

    let operation = classify_command(executable, normalized);

    Ok(ParsedCommand {
        operation,
        command: command.to_string(),
    })
}

fn classify_command(executable: &str, command: &str) -> Operation {
    match executable {
        "useradd" => Operation::UserCreate,

        "usermod" => Operation::UserModify,

        "userdel" => Operation::UserDelete,

        "groupadd" => Operation::GroupCreate,

        "groupmod" => Operation::GroupModify,

        "groupdel" => Operation::GroupDelete,

        "apt" | "apt-get" | "dnf" | "yum" | "apk" | "pacman" | "zypper" => {
            classify_package_command(command)
        }

        "systemctl" => classify_systemctl_command(command),

        "service" => classify_service_command(command),

        "cat" | "echo" | "less" | "more" | "head" | "tail" | "grep" | "awk"
        | "sed" | "find" | "ls" | "pwd" | "who" | "w" | "id"
        | "uname" | "df" | "du" | "free" | "ps" | "ip" | "ss" => {
            Operation::ReadOnly
        }

        "curl" | "wget" => Operation::NetworkRead,

        "rm" | "rmdir" => Operation::FileDelete,

        "touch" | "mkdir" | "cp" | "mv" | "tee" => Operation::FileWrite,

        "reboot" | "shutdown" | "poweroff" | "halt" => {
            Operation::SystemShutdown
        }

        _ => Operation::Unknown,
    }
}

fn classify_package_command(command: &str) -> Operation {
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.iter().any(|part| {
        matches!(
            *part,
            "install" | "add"
        )
    }) {
        return Operation::PackageInstall;
    }

    if parts.iter().any(|part| {
        matches!(
            *part,
            "remove" | "erase" | "uninstall" | "del"
        )
    }) {
        return Operation::PackageRemove;
    }

    if parts.iter().any(|part| {
        matches!(
            *part,
            "update" | "upgrade"
        )
    }) {
        return Operation::PackageUpdate;
    }

    Operation::Unknown
}

fn classify_systemctl_command(command: &str) -> Operation {
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.contains(&"start") {
        return Operation::ServiceStart;
    }

    if parts.contains(&"stop") {
        return Operation::ServiceStop;
    }

    if parts.contains(&"restart") {
        return Operation::ServiceRestart;
    }

    if parts.contains(&"enable") {
        return Operation::ServiceEnable;
    }

    if parts.contains(&"disable") {
        return Operation::ServiceDisable;
    }

    if parts.contains(&"status") {
        return Operation::ReadOnly;
    }

    Operation::Unknown
}

fn classify_service_command(command: &str) -> Operation {
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.contains(&"start") {
        return Operation::ServiceStart;
    }

    if parts.contains(&"stop") {
        return Operation::ServiceStop;
    }

    if parts.contains(&"restart") {
        return Operation::ServiceRestart;
    }

    if parts.contains(&"status") {
        return Operation::ReadOnly;
    }

    Operation::Unknown
}

fn reject_shell_constructs(command: &str) -> Result<()> {
    let dangerous = [
        "&&",
        "||",
        ";",
        "|",
        "`",
        "$(",
        "\n",
        "\r",
    ];

    for construct in dangerous {
        if command.contains(construct) {
            bail!(
                "Shell construct '{}' is not permitted",
                construct
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_user_creation() {
        let result = parse("sudo useradd john").unwrap();

        assert_eq!(
            result.operation,
            Operation::UserCreate
        );
    }

    #[test]
    fn detects_package_install() {
        let result =
            parse("sudo apt install nginx").unwrap();

        assert_eq!(
            result.operation,
            Operation::PackageInstall
        );
    }

    #[test]
    fn detects_dnf_install() {
        let result =
            parse("sudo dnf install nginx").unwrap();

        assert_eq!(
            result.operation,
            Operation::PackageInstall
        );
    }

    #[test]
    fn detects_service_restart() {
        let result =
            parse("sudo systemctl restart nginx").unwrap();

        assert_eq!(
            result.operation,
            Operation::ServiceRestart
        );
    }

    #[test]
    fn detects_group_modification() {
        let result = parse("sudo groupmod -n admins developers").unwrap();

        assert_eq!(
            result.operation,
            Operation::GroupModify
        );
    }

    #[test]
    fn detects_network_read() {
        let result = parse("curl https://example.com").unwrap();

        assert_eq!(
            result.operation,
            Operation::NetworkRead
        );
    }

    #[test]
    fn detects_echo_exit_status() {
        let result = parse("echo $?").unwrap();

        assert_eq!(
            result.operation,
            Operation::ReadOnly
        );
    }

    #[test]
    fn rejects_command_chaining() {
        let result =
            parse("sudo systemctl restart nginx && rm -rf /");

        assert!(result.is_err());
    }
}