#!/usr/bin/env python3
"""Record and verify the Spec 00 frozen-contract checksums (development tooling only).

    python scripts/verify_contracts_freeze.py write    # (re)write specs/contracts/FROZEN.sha256
    python scripts/verify_contracts_freeze.py verify   # fail if any frozen file changed

Frozen set: everything under crates/contracts/ (excluding build output) and the
specs/contracts/*.md design records except HANDOFF.md. Hashes are over file bytes with
CRLF normalized to LF so a Windows checkout and a Unix checkout agree. The manifest's own
digest is printed (and recorded in HANDOFF.md); HANDOFF.md is excluded so it can cite it.

Changing a frozen file is a contract change: follow the compatibility policy in
specs/contracts/HANDOFF.md, then run `write` and update HANDOFF.md.
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "specs" / "contracts" / "FROZEN.sha256"
SKIP_DIRS = {"target", "__pycache__", ".git"}
EXCLUDED = {"specs/contracts/HANDOFF.md", "specs/contracts/FROZEN.sha256"}


def frozen_files() -> list[Path]:
    files: list[Path] = []
    for path in sorted((ROOT / "crates" / "contracts").rglob("*")):
        if path.is_file() and not (set(path.relative_to(ROOT).parts) & SKIP_DIRS):
            files.append(path)
    for path in sorted((ROOT / "specs" / "contracts").glob("*.md")):
        if path.relative_to(ROOT).as_posix() not in EXCLUDED:
            files.append(path)
    return files


def digest(path: Path) -> str:
    data = path.read_bytes().replace(b"\r\n", b"\n")
    return hashlib.sha256(data).hexdigest()


def render() -> str:
    lines = [f"{digest(p)}  {p.relative_to(ROOT).as_posix()}" for p in frozen_files()]
    return "\n".join(lines) + "\n"


def manifest_digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def main(argv: list[str]) -> int:
    if len(argv) != 2 or argv[1] not in {"write", "verify"}:
        print(__doc__)
        return 2
    current = render()
    if argv[1] == "write":
        MANIFEST.write_text(current, encoding="utf-8", newline="\n")
        print(f"wrote {MANIFEST.relative_to(ROOT)}: {len(current.splitlines())} files")
        print(f"manifest sha256: {manifest_digest(current)}")
        return 0
    if not MANIFEST.exists():
        print("missing manifest; run `write` first")
        return 1
    recorded = MANIFEST.read_text(encoding="utf-8").replace("\r\n", "\n")
    if recorded == current:
        print(f"frozen contracts verified: {len(current.splitlines())} files")
        print(f"manifest sha256: {manifest_digest(current)}")
        return 0
    old = dict(l.split("  ", 1)[::-1] for l in recorded.splitlines())
    new = dict(l.split("  ", 1)[::-1] for l in current.splitlines())
    for name in sorted(set(old) | set(new)):
        if old.get(name) != new.get(name):
            status = "added" if name not in old else "removed" if name not in new else "changed"
            print(f"{status}: {name}")
    print("frozen contract files differ from the manifest")
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
