// A command is executed as root, over the same SSH key, whenever it is invoked through sudo.
pub fn is_sudo_command(command: &str) -> bool {
    command.trim().split_whitespace().next() == Some("sudo")
}

pub fn ensure_non_interactive_sudo(command: &str) -> String {
    let command = command.trim();
    let Some(arguments) = command.strip_prefix("sudo") else {
        return command.to_string();
    };

    if arguments.is_empty()
        || !arguments.starts_with(char::is_whitespace)
        || command
            .split_whitespace()
            .any(|argument| argument == "-n" || argument == "--non-interactive")
    {
        return command.to_string();
    }

    format!("sudo -n{arguments}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sudo_command() {
        assert!(is_sudo_command("sudo apt update"));
    }

    #[test]
    fn does_not_detect_sudo_in_non_sudo_command() {
        assert!(!is_sudo_command("cat /etc/os-release"));
    }

    #[test]
    fn does_not_detect_sudo_as_substring() {
        assert!(!is_sudo_command("sudoedit /etc/hosts"));
    }

    #[test]
    fn adds_non_interactive_flag_to_sudo_command() {
        assert_eq!(
            ensure_non_interactive_sudo("sudo apt update"),
            "sudo -n apt update"
        );
    }

    #[test]
    fn preserves_existing_non_interactive_flag() {
        assert_eq!(
            ensure_non_interactive_sudo("sudo -n apt update"),
            "sudo -n apt update"
        );
    }

    #[test]
    fn supports_sudo_options() {
        assert_eq!(
            ensure_non_interactive_sudo("sudo -u root useradd alice"),
            "sudo -n -u root useradd alice"
        );
    }

    #[test]
    fn preserves_non_sudo_command() {
        assert_eq!(
            ensure_non_interactive_sudo("getent passwd"),
            "getent passwd"
        );
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            ensure_non_interactive_sudo("  sudo useradd alice  "),
            "sudo -n useradd alice"
        );
    }

    #[test]
    fn preserves_incomplete_sudo_command() {
        assert_eq!(ensure_non_interactive_sudo("sudo"), "sudo");
    }
}