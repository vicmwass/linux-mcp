use anyhow::{Context, Result, ensure};

/// The password is transported only as stdin, never as a command argument.
pub(super) fn prepare(command: &str, password_file: Option<&str>) -> Result<(String, Vec<u8>)> {
    match password_file {
        Some(path) => {
            let password = std::fs::read_to_string(path)
                .context("Failed to read configured sudo password file")?;
            // A NOPASSWD rule may leave stdin unread. Never let the target
            // command consume the password if sudo does not need it.
            let script = format!("exec {command} </dev/null");
            let script = format!("'{}'", script.replace('\'', "'\\''"));
            Ok((
                format!("sudo -k -S -p '' -- sh -c {script}"),
                password_stdin(&password)?,
            ))
        }
        None => Ok((format!("sudo -n -- {command}"), Vec::new())),
    }
}

fn password_stdin(secret: &str) -> Result<Vec<u8>> {
    let secret = secret.trim_end_matches(['\r', '\n']);
    ensure!(
        !secret.is_empty() && !secret.contains(['\r', '\n', '\0']),
        "Invalid sudo secret: expected one nonempty line"
    );
    Ok(format!("{secret}\n").into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_password_spaces_and_shell_metacharacters() {
        assert_eq!(password_stdin(" p'$word ").unwrap(), b" p'$word \n");
    }

    #[test]
    fn accepts_windows_line_ending() {
        assert_eq!(
            password_stdin("test-password\r\n").unwrap(),
            b"test-password\n"
        );
    }

    #[test]
    fn rejects_empty_multiline_and_nul_without_disclosing_password() {
        for secret in [
            "",
            "\r\n",
            "sensitive\nsecond",
            "sensitive\rsecond",
            "sensitive\0",
        ] {
            let error = password_stdin(secret).unwrap_err().to_string();
            assert!(!error.contains("sensitive"));
        }
    }

    #[test]
    fn absent_secret_uses_noninteractive_sudo() {
        let (command, stdin) = prepare("'id' '-u'", None).unwrap();
        assert_eq!(command, "sudo -n -- 'id' '-u'");
        assert!(stdin.is_empty());
    }

    #[test]
    fn configured_missing_secret_fails_instead_of_falling_back() {
        let path =
            std::env::temp_dir().join(format!("linux-mcp-missing-{}-sudo", std::process::id()));
        assert!(prepare("id", Some(path.to_str().unwrap())).is_err());
    }
}
