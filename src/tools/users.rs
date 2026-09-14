use std::time::Duration;

use anyhow::Result;

use crate::{
    commands::ensure_non_interactive_sudo,
    ssh::SshManager,
};

pub async fn list_users(
    ssh_manager: &SshManager,
    server_id: &str,
) -> Result<String> {
    let result = ssh_manager
        .execute(server_id, "getent passwd", Duration::from_secs(30))
        .await?;

    if result.exit_code != 0 {
        anyhow::bail!(
            "Failed to retrieve users: {}",
            result.stderr.trim()
        );
    }

    let users = result
        .stdout
        .lines()
        .filter_map(parse_passwd_entry)
        .map(|user| {
            format!(
                "Username: {}\nUID: {}\nGID: {}\nHome: {}\nShell: {}",
                user.username,
                user.uid,
                user.gid,
                user.home,
                user.shell
            )
        })
        .collect::<Vec<_>>();

    if users.is_empty() {
        return Ok("No users found.".to_string());
    }

    Ok(users.join("\n\n"))
}



pub async fn get_user(
    ssh_manager: &SshManager,
    server_id: &str,
    username: &str,
) -> Result<String> {
    if username.trim().is_empty() {
        anyhow::bail!("Username cannot be empty");
    }

    // We use getent so this works with the server's configured
    // NSS sources rather than only /etc/passwd.
    let command = format!(
        "getent passwd {}",
        shell_escape(username)
    );

    let result = ssh_manager
        .execute(server_id, &command, Duration::from_secs(30))
        .await?;

    if result.exit_code != 0 {
        anyhow::bail!(
            "User '{}' was not found: {}",
            username,
            result.stderr.trim()
        );
    }

    match result.stdout.lines().next() {
        Some(line) => {
            let user = parse_passwd_entry(line)
                .ok_or_else(|| anyhow::anyhow!("Invalid passwd entry returned by server"))?;

            Ok(format!(
                "Username: {}\nUID: {}\nGID: {}\nHome: {}\nShell: {}",
                user.username,
                user.uid,
                user.gid,
                user.home,
                user.shell
            ))
        }

        None => {
            anyhow::bail!("User '{}' was not found", username);
        }
    }
}

pub async fn delete_user(
    ssh_manager: &SshManager,
    server_id: &str,
    username: &str,
    confirm: bool,
    remove_home: bool,
) -> Result<String> {
    validate_username(username)?;

    if !confirm {
        anyhow::bail!(
            "User deletion requires explicit confirmation. Set confirm=true."
        );
    }

    // Never allow deletion of critical system accounts.
    let protected_users = [
        "root",
        "daemon",
        "bin",
        "sys",
        "sync",
        "games",
        "man",
        "lp",
        "mail",
        "news",
        "uucp",
        "proxy",
        "www-data",
        "backup",
        "list",
        "irc",
        "nobody",
        "systemd-network",
        "systemd-timesync",
        "messagebus",
        "sshd",
    ];

    if protected_users.contains(&username) {
        anyhow::bail!(
            "Deletion of protected system user '{}' is not permitted",
            username
        );
    }

    // Make sure the user exists and retrieve its information.
    let check_command = format!(
        "getent passwd {}",
        shell_escape(username)
    );

    let check_result = ssh_manager
        .execute(server_id, &check_command, Duration::from_secs(30))
        .await?;

    if check_result.exit_code != 0 || check_result.stdout.trim().is_empty() {
        anyhow::bail!("User '{}' does not exist", username);
    }

    // Check whether the user currently has active sessions.
    let session_command = format!(
        "who | awk '{{print $1}}' | grep -Fx {}",
        shell_escape(username)
    );

    let session_result = ssh_manager
        .execute(server_id, &session_command, Duration::from_secs(30))
        .await?;

    if interpret_session_check(
        session_result.exit_code,
        &session_result.stdout,
    )? {
        anyhow::bail!(
            "User '{}' currently has an active session. \
             Terminate the session before deleting the account.",
            username
        );
    }

    let mut command = String::from("sudo userdel");

    if remove_home {
        command.push_str(" -r");
    }

    command.push(' ');
    command.push_str(&shell_escape(username));
    let command = ensure_non_interactive_sudo(&command);

    let result = ssh_manager
        .execute(server_id, &command, Duration::from_secs(30) )
        .await?;

    if result.exit_code != 0 {
        anyhow::bail!(
            "Failed to delete user '{}': {}",
            username,
            result.stderr.trim()
        );
    }

    Ok(format!(
        "User '{}' has been permanently deleted{}.",
        username,
        if remove_home {
            " along with their home directory"
        } else {
            ""
        }
    ))
}
pub async fn enable_user(
    ssh_manager: &SshManager,
    server_id: &str,
    username: &str,
) -> Result<String> {
    validate_username(username)?;

    // Make sure the user exists.
    let check_command = format!(
        "getent passwd {}",
        shell_escape(username)
    );

    let check_result = ssh_manager
        .execute(server_id, &check_command, Duration::from_secs(30))
        .await?;

    if check_result.exit_code != 0 || check_result.stdout.trim().is_empty() {
        anyhow::bail!("User '{}' does not exist", username);
    }

    let command = format!(
        "sudo usermod -U {}",
        shell_escape(username)
    );
    let command = ensure_non_interactive_sudo(&command);

    let result = ssh_manager
        .execute(server_id, &command, Duration::from_secs(30))
        .await?;

    if result.exit_code != 0 {
        anyhow::bail!(
            "Failed to enable user '{}': {}",
            username,
            result.stderr.trim()
        );
    }

    Ok(format!(
        "User '{}' has been enabled successfully.",
        username
    ))
}

pub async fn create_user(
    ssh_manager: &SshManager,
    server_id: &str,
    username: &str,
    home: Option<&str>,
    shell: Option<&str>,
) -> Result<String> {
    validate_username(username)?;

    if let Some(home) = home {
        validate_path(home)?;
    }

    if let Some(shell) = shell {
        validate_shell(shell)?;
    }

    let mut command = String::from("sudo useradd");

    if let Some(home) = home {
        command.push_str(&format!(" -d {}", shell_escape(home)));
    }

    if let Some(shell) = shell {
        command.push_str(&format!(" -s {}", shell_escape(shell)));
    }

    command.push_str(&format!(" {}", shell_escape(username)));
    let command = ensure_non_interactive_sudo(&command);

    let result = ssh_manager
        .execute(server_id, &command, Duration::from_secs(30))
        .await?;

    if result.exit_code != 0 {
        anyhow::bail!(
            "Failed to create user '{}': {}",
            username,
            result.stderr.trim()
        );
    }

    Ok(format!("User '{}' created successfully.", username))
}

pub async fn disable_user(
    ssh_manager: &SshManager,
    server_id: &str,
    username: &str,
) -> Result<String> {
    validate_username(username)?;

    // Make sure the user exists before attempting to disable it.
    let check_command = format!(
        "getent passwd {}",
        shell_escape(username)
    );

    let check_result = ssh_manager
        .execute(server_id, &check_command, Duration::from_secs(30))
        .await?;

    if check_result.exit_code != 0 || check_result.stdout.trim().is_empty() {
        anyhow::bail!("User '{}' does not exist", username);
    }

    let command = format!(
        "sudo usermod -L {}",
        shell_escape(username)
    );
    let command = ensure_non_interactive_sudo(&command);

    let result = ssh_manager
        .execute(server_id, &command, Duration::from_secs(30)   )
        .await?;

    if result.exit_code != 0 {
        anyhow::bail!(
            "Failed to disable user '{}': {}",
            username,
            result.stderr.trim()
        );
    }

    Ok(format!(
        "User '{}' has been disabled successfully.",
        username
    ))
}

fn validate_username(username: &str) -> Result<()> {
    if username.is_empty() {
        anyhow::bail!("Username cannot be empty");
    }

    if username.len() > 32 {
        anyhow::bail!("Username cannot exceed 32 characters");
    }

    if !username
        .chars()
        .next()
        .map(|c| c.is_ascii_lowercase() || c == '_')
        .unwrap_or(false)
    {
        anyhow::bail!(
            "Username must start with a lowercase letter or underscore"
        );
    }

    if !username
        .chars()
        .all(|c| c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || c == '_'
            || c == '-')
    {
        anyhow::bail!(
            "Username may only contain lowercase letters, numbers, '-' and '_'"
        );
    }

    Ok(())
}

fn validate_path(path: &str) -> Result<()> {
    if path.is_empty() {
        anyhow::bail!("Home directory cannot be empty");
    }

    // useradd rejects a relative -d value with a confusing remote error.
    if !path.starts_with('/') {
        anyhow::bail!("Home directory must be an absolute path, e.g. /home/{path}");
    }

    if path.contains('\n')
        || path.contains('\r') 
    {
        anyhow::bail!("Invalid characters in home directory");
    }

    Ok(())
}

fn validate_shell(shell: &str) -> Result<()> {
    let allowed_shells = [
        "/bin/bash",
        "/bin/sh",
        "/bin/zsh",
        "/bin/fish",
        "/bin/dash",
    ];

    if !allowed_shells.contains(&shell) {
        anyhow::bail!(
            "Unsupported login shell. Allowed shells: {}",
            allowed_shells.join(", ")
        );
    }

    Ok(())
}

struct User {
    username: String,
    uid: String,
    gid: String,
    home: String,
    shell: String,
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn parse_passwd_entry(line: &str) -> Option<User> {
    let fields: Vec<&str> = line.split(':').collect();

    if fields.len() < 7 {
        return None;
    }

    Some(User {
        username: fields[0].to_string(),
        uid: fields[2].to_string(),
        gid: fields[3].to_string(),
        home: fields[5].to_string(),
        shell: fields[6].to_string(),
    })
}

fn interpret_session_check(exit_code: u32, stdout: &str) -> Result<bool> {
    match exit_code {
        0 => Ok(!stdout.trim().is_empty()),
        1 => Ok(false),
        _ => anyhow::bail!(
            "Failed to check active user sessions (exit code {exit_code})"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_check_reports_active_session_for_matching_output() {
        assert!(interpret_session_check(0, "alice\n").unwrap());
    }

    #[test]
    fn session_check_accepts_no_matching_session() {
        assert!(!interpret_session_check(1, "").unwrap());
    }

    #[test]
    fn session_check_rejects_probe_errors() {
        assert!(interpret_session_check(2, "").is_err());
    }

    #[test]
    fn validate_path_accepts_absolute_path() {
        assert!(validate_path("/home/vicmwass").is_ok());
    }

    #[test]
    fn validate_path_rejects_relative_path() {
        assert!(validate_path("vicmwass").is_err());
    }
}