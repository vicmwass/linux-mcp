"""Opt-in mutation test: creates and removes one temporary Ubuntu test account.

The account has no usable password, no SSH keys and no created home directory.
"""
import argparse
from pathlib import Path
import tomllib
import uuid

from verify_sudo import Client


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--server", required=True)
    parser.add_argument("--connection", choices=["linux-mcp", "MCP_DOCKER"], required=True)
    args = parser.parse_args()
    config = tomllib.loads((Path.home() / ".codex/config.toml").read_text(encoding="utf-8-sig"))
    client = Client(config["mcp_servers"][args.connection])
    username = "mcpcheck_" + uuid.uuid4().hex[:12]
    account = {"server_id": args.server, "username": username}
    created = False
    try:
        result = client.call("create_user", {**account, "home": "/nonexistent", "shell": "/bin/sh"})
        assert "created successfully" in result, result
        created = True
        # '*' is not a password hash and cannot authenticate. It lets us verify
        # lock/unlock without usermod rejecting an empty password field.
        result = client.call("execute_command", {
            "server_id": args.server, "executable": "usermod", "args": ["-p", "*", username],
            "privilege": "sudo", "confirm": True,
        })
        assert "Exit code: 0" in result, result
        for tool, success in [("disable_user", "disabled successfully"), ("enable_user", "enabled successfully")]:
            result = client.call(tool, account)
            assert success in result, result
        print(f"PASS {args.connection} / {args.server}: create, disable and enable temporary account", flush=True)
    finally:
        try:
            if created:
                result = client.call("delete_user", {**account, "confirm": True, "remove_home": False})
                assert "permanently deleted" in result, f"Cleanup failed for {username}: {result}"
                result = client.call("get_user", account)
                assert "was not found" in result, f"Account still present: {username}"
                print("PASS temporary account deleted and absence verified", flush=True)
        finally:
            client.close()


if __name__ == "__main__":
    main()
