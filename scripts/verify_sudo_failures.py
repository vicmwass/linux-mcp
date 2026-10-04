"""Check the deployed direct image with isolated synthetic sudo secrets.

Does not change real inventory, passwords, or remote accounts.
"""
import copy
from pathlib import Path
import re
import tempfile
import tomllib

from verify_sudo import Client


def main():
    config = tomllib.loads((Path.home() / ".codex/config.toml").read_text(encoding="utf-8-sig"))
    launcher = config["mcp_servers"]["linux-mcp"]
    original_mount = next(arg for arg in launcher["args"] if arg.endswith(":/app/config:ro"))
    inventory = (Path(original_mount.removesuffix(":/app/config:ro")) / "servers.toml").read_text()
    inventory, count = re.subn(r'(?m)^sudo_password_file\s*=.*$',
                              'sudo_password_file = "/app/config/test-password"', inventory)
    assert count > 0
    with tempfile.TemporaryDirectory(prefix="linux-mcp-negative-") as directory:
        root = Path(directory)
        (root / "servers.toml").write_text(inventory, encoding="utf-8")
        isolated = copy.deepcopy(launcher)
        isolated["args"] = [f"{root.as_posix()}:/app/config:ro" if arg == original_mount else arg
                            for arg in isolated["args"]]
        for case, contents, expected in [
            ("missing", None, "Failed to read configured sudo password file"),
            ("empty", "", "Invalid sudo secret"),
            ("multiline", "synthetic\nsecond", "Invalid sudo secret"),
            ("incorrect", "linux-mcp-deliberately-invalid-password", "Exit code: 1"),
        ]:
            if contents is not None:
                (root / "test-password").write_text(contents, encoding="utf-8")
            client = Client(isolated)
            try:
                result = client.call("execute_command", {
                    "server_id": "ubuntu-01", "executable": "id", "args": ["-u"],
                    "privilege": "sudo", "confirm": True,
                })
                assert expected in result, f"Unexpected result for {case} password"
                assert "STDOUT:\n0\n" not in result, f"Unexpected elevation with {case} password"
                if contents:
                    assert contents not in result, "Synthetic password was disclosed"
                print(f"PASS {case} sudo password: rejected without fallback or disclosure", flush=True)
            finally:
                client.close()


if __name__ == "__main__":
    main()
