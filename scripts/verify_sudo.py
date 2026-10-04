"""Read-only MCP deployment checks using the configured Codex stdio entry.

Usage: python scripts/verify_sudo.py --connection linux-mcp
       python scripts/verify_sudo.py --connection MCP_DOCKER
No passwords are read or printed by this client.
"""
import argparse
import json
import os
from pathlib import Path
import queue
import subprocess
import threading
import time
import tomllib


class Client:
    def __init__(self, config):
        self.responses = queue.Queue()
        self.sequence = 0
        self.process = subprocess.Popen(
            [config["command"], *config.get("args", [])],
            cwd=config.get("cwd"),
            env={**os.environ, **config.get("env", {})},
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, encoding="utf-8",
        )
        threading.Thread(target=self.read, daemon=True).start()
        self.request("initialize", {
            "protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "linux-mcp-sudo-check", "version": "1.0"},
        })
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def read(self):
        for line in self.process.stdout:
            try:
                self.responses.put(json.loads(line))
            except json.JSONDecodeError:
                pass  # Docker gateway startup diagnostics are not JSON-RPC.
        self.responses.put(None)

    def send(self, message):
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def request(self, method, params):
        self.sequence += 1
        self.send({"jsonrpc": "2.0", "id": self.sequence, "method": method, "params": params})
        deadline = time.monotonic() + 120
        while True:
            response = self.responses.get(timeout=max(0.01, deadline - time.monotonic()))
            if response is None:
                raise RuntimeError("MCP process closed unexpectedly")
            if response.get("id") != self.sequence:
                continue
            if "error" in response:
                raise RuntimeError(response["error"])
            return response["result"]

    def call(self, name, arguments):
        result = self.request("tools/call", {"name": name, "arguments": arguments})
        if result.get("isError"):
            raise RuntimeError("MCP tool reported an error")
        return "\n".join(item.get("text", "") for item in result.get("content", []))

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=15)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            self.process.wait(timeout=10)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--connection", choices=["linux-mcp", "MCP_DOCKER"], required=True)
    parser.add_argument("--config", type=Path, default=Path.home() / ".codex/config.toml")
    parser.add_argument("--servers", nargs="+", default=["ubuntu-01", "linux-mint"])
    args = parser.parse_args()
    config_text = args.config.read_text(encoding="utf-8-sig")
    config = json.loads(config_text) if args.config.suffix.lower() == ".json" else tomllib.loads(config_text)
    entries = config.get("mcpServers", config.get("mcp_servers", {}))
    client = Client(entries[args.connection])
    try:
        tools = client.request("tools/list", {})["tools"]
        assert any(tool["name"] == "execute_command" for tool in tools)
        for server in args.servers:
            base = {"server_id": server, "executable": "id", "args": ["-u"], "privilege": "sudo"}
            refusal = client.call("execute_command", base)
            assert "requires explicit confirmation" in refusal, refusal
            elevated = client.call("execute_command", {**base, "confirm": True})
            assert "Exit code: 0\nSTDOUT:\n0\n" in elevated, elevated
            ordinary = client.call("execute_command", {**base, "privilege": "user", "confirm": True})
            assert "Exit code: 0" in ordinary and "STDOUT:\n0\n" not in ordinary, ordinary
            # cat must see EOF, never the sudo password (also under NOPASSWD).
            isolated = client.call("execute_command", {**base, "executable": "cat", "args": [], "confirm": True})
            assert isolated.rstrip("\r\n") == "Exit code: 0\nSTDOUT:\n\nSTDERR:", "Elevated stdin isolation check failed"
            users = client.call("list_users", {"server_id": server})
            assert "Username: root" in users, users
            print(f"PASS {args.connection} / {server}: confirmation, sudo uid=0, ordinary uid, isolated stdin, list_users", flush=True)
    finally:
        client.close()


if __name__ == "__main__":
    main()
