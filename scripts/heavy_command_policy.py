#!/usr/bin/env python3
"""Shared agent-command policy for the local CPU budget.

Usage:
    python3 scripts/heavy_command_policy.py --command <shell command>
    <hook JSON> | python3 scripts/heavy_command_policy.py
    python3 scripts/heavy_command_policy.py --self-check

Refuses a bare heavy command before it runs, with exit 2 and the reason on
stderr, so the opencode plugin, Claude Code, and Codex share one matcher and one
message. Exit 0 allows the command (including non-Bash hook input). The hook
JSON carries the command at tool_input.command, which is what Claude Code and
Codex send.

The refusal names the wrapper under the current checkout's root, so the hint
works from a subdirectory or a linked worktree.
"""
from __future__ import annotations

import json
import os
import re
import shlex
import subprocess
import sys
from typing import Optional, Sequence

HEAVY_SEGMENT = re.compile(r"^\s*(cargo|cmake|ctest|ninja)\b")
CHEAP_SEGMENT = re.compile(r"^\s*cargo\s+fmt\b")
COMPOUND = re.compile(r"[;&|]|\b(?:bash|sh)\s+-c\b")
EXEMPT = (
    "cpu-budget.py",
    "with-build-lock.sh",
    "verify-fast.sh",
    "verify-full.sh",
    "test-report.sh",
)
USAGE = (
    "usage: heavy_command_policy.py --command <shell command>\n"
    "       <hook JSON> | heavy_command_policy.py\n"
    "       heavy_command_policy.py --self-check"
)


def repo_root(start: Optional[str] = None) -> str:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=start or os.getcwd(),
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError:
        result = None
    if result is not None and result.returncode == 0 and result.stdout.strip():
        return result.stdout.strip()
    return start or os.getcwd()


def wrapper_path(root: str) -> str:
    if os.path.realpath(root) == os.path.realpath(os.getcwd()):
        return "scripts/with-build-lock.sh"
    return os.path.join(root, "scripts", "with-build-lock.sh")


def refusal(command: str, root: Optional[str] = None) -> Optional[str]:
    if not isinstance(command, str) or command.strip() == "":
        return None
    if any(name in command for name in EXEMPT):
        return None
    # ponytail: segment match only; env-prefixed, absolute-path, and
    # shell-constructed invocations bypass it. Prose and the wrapper remain the
    # baseline.
    heavy = any(
        HEAVY_SEGMENT.match(segment) and not CHEAP_SEGMENT.match(segment)
        for segment in re.split(r"[;&|\n]", command)
    )
    if not heavy:
        return None
    wrapper = shlex.quote(wrapper_path(root or repo_root()))
    if COMPOUND.search(command):
        wrapped = f"bash {wrapper} -- bash -c {shlex.quote(command)}"
    else:
        wrapped = f"bash {wrapper} -- {command}"
    return (
        "Refused a bare heavy command. Run it through the CPU-budget wrapper:\n"
        f"  {wrapped}"
    )


def command_from(stdin_text: str) -> Optional[str]:
    if stdin_text.strip() == "":
        return None
    try:
        payload = json.loads(stdin_text)
    except ValueError:
        return None
    if not isinstance(payload, dict):
        return None
    tool = payload.get("tool_name")
    if isinstance(tool, str) and tool.lower() not in ("bash", "exec_command"):
        return None
    tool_input = payload.get("tool_input")
    if not isinstance(tool_input, dict):
        return None
    command = tool_input.get("command")
    return command if isinstance(command, str) else None


def self_check() -> int:
    for command in (
        "cargo build",
        "cargo nextest run --workspace",
        "cmake -S . -B build",
        "ctest --test-dir build",
        "ninja -C build",
        "git status && cargo test",
        "true; ctest",
        "ninja -C build | tee log",
        "cargo fmt --all && cargo build",
    ):
        message = refusal(command, "/tmp/checkout")
        assert message is not None, command
        assert "with-build-lock.sh" in message, message
    assert "/tmp/checkout/scripts/with-build-lock.sh" in refusal(
        "cargo build", "/tmp/checkout"
    )
    assert "bash -c" in refusal("git status && cargo test", "/tmp/checkout")
    assert "scripts/with-build-lock.sh" in refusal("cargo build")
    for command in (
        "bash scripts/with-build-lock.sh -- cargo build",
        "python3 scripts/cpu-budget.py exec -- cargo test",
        "bash scripts/verify-fast.sh",
        "bash scripts/verify-full.sh",
        "bash scripts/test-report.sh never",
        "cargo fmt --all",
        "cargo fmt && git status",
        "git status",
        "echo cargo",
    ):
        assert refusal(command) is None, command
    assert command_from("") is None
    assert command_from("not json") is None
    assert command_from('{"tool_name":"Read","tool_input":{"file_path":"x"}}') is None
    assert (
        command_from('{"tool_name":"Bash","tool_input":{"command":"cargo build"}}')
        == "cargo build"
    )
    assert (
        command_from('{"tool_name":"exec_command","tool_input":{"command":"cargo build"}}')
        == "cargo build"
    )

    def run(arguments: Sequence[str], stdin_text: str = "") -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, os.path.abspath(__file__), *arguments],
            input=stdin_text,
            capture_output=True,
            text=True,
            check=False,
        )

    refused = run([], '{"tool_name":"Bash","tool_input":{"command":"cargo build"}}')
    assert refused.returncode == 2 and "with-build-lock.sh" in refused.stderr, (
        refused.returncode,
        refused.stderr,
    )
    direct = run(["--command", "cargo build"])
    assert direct.returncode == 2 and "with-build-lock.sh" in direct.stderr
    for arguments, stdin_text in (
        ([], ""),
        ([], "not json"),
        ([], '{"tool_name":"Read","tool_input":{"file_path":"x"}}'),
        ([], '{"tool_name":"Bash","tool_input":{"command":"bash scripts/with-build-lock.sh -- cargo build"}}'),
        ([], '{"tool_name":"Bash","tool_input":{"command":"cargo fmt --all"}}'),
        (["--command", "git status"], ""),
    ):
        allowed = run(arguments, stdin_text)
        assert allowed.returncode == 0, (arguments, stdin_text, allowed.stderr)
    print("heavy-command-policy: self-check OK")
    return 0


def main(argv: Sequence[str]) -> int:
    if "--self-check" in argv:
        return self_check()
    if argv and argv[0] in ("-h", "--help"):
        print(USAGE)
        return 0
    if argv and argv[0] == "--command":
        if len(argv) < 2:
            print(USAGE, file=sys.stderr)
            return 2
        command: Optional[str] = argv[1]
    else:
        command = command_from(sys.stdin.read())
    if command is None:
        return 0
    message = refusal(command)
    if message is None:
        return 0
    print(message, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
