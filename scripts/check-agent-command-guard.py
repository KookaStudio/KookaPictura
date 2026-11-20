#!/usr/bin/env python3
"""Verify the committed agent command guard is wired for every harness.

Reads the three adapters and the fast gate and fails when:
- `.claude/settings.json` or `.codex/hooks.json` lacks a `PreToolUse` group
  matching `Bash` with a command handler that runs
  `scripts/heavy_command_policy.py`,
- the handler has no timeout of at most 30 seconds,
- the Claude handler does not resolve `${CLAUDE_PROJECT_DIR}`, does not set
  `onFailure: "block"`, or the Codex handler does not resolve the main checkout
  through `--git-common-dir`,
- the opencode plugin does not delegate to the policy, or a second matcher
  (`.opencode/lib/heavy-command.mjs`) has come back,
- `scripts/verify-fast.sh` does not run the policy self-check and this check.

Run from the repository root: python3 scripts/check-agent-command-guard.py
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Optional

ROOT = Path(__file__).resolve().parent.parent
POLICY = "heavy_command_policy.py"


def find_handler(
    document: dict[str, Any], event: str = "PreToolUse", matcher: str = "Bash"
) -> Optional[dict[str, Any]]:
    groups = document.get("hooks", {}).get(event, [])
    if not isinstance(groups, list):
        return None
    for group in groups:
        if not isinstance(group, dict) or group.get("matcher") != matcher:
            continue
        handlers = group.get("hooks", [])
        if not isinstance(handlers, list):
            continue
        for handler in handlers:
            if isinstance(handler, dict) and handler.get("type") == "command":
                return handler
    return None


def validate(
    claude: dict[str, Any],
    codex: dict[str, Any],
    plugin_text: str,
    fast_text: str,
    legacy_matcher: bool,
) -> list[str]:
    errors: list[str] = []

    claude_hook = find_handler(claude)
    if claude_hook is None:
        errors.append(".claude/settings.json: no PreToolUse Bash command hook")
    else:
        command = str(claude_hook.get("command", ""))
        if POLICY not in command:
            errors.append(".claude/settings.json: hook does not run the policy")
        if "CLAUDE_PROJECT_DIR" not in command:
            errors.append(".claude/settings.json: hook does not use CLAUDE_PROJECT_DIR")
        if claude_hook.get("onFailure") != "block":
            errors.append(".claude/settings.json: onFailure is not block")
        timeout = claude_hook.get("timeout")
        if not isinstance(timeout, (int, float)) or timeout > 30:
            errors.append(".claude/settings.json: missing or oversized timeout")

    codex_hook = find_handler(codex)
    if codex_hook is None:
        errors.append(".codex/hooks.json: no PreToolUse Bash command hook")
    else:
        command = str(codex_hook.get("command", ""))
        if POLICY not in command:
            errors.append(".codex/hooks.json: hook does not run the policy")
        if "--show-toplevel" not in command or "--git-common-dir" not in command:
            errors.append(".codex/hooks.json: hook does not resolve the checkout and its worktree fallback")
        timeout = codex_hook.get("timeout")
        if not isinstance(timeout, (int, float)) or timeout > 30:
            errors.append(".codex/hooks.json: missing or oversized timeout")

    if POLICY not in plugin_text:
        errors.append(".opencode/plugins/cpu-budget.ts: does not delegate to the policy")
    if legacy_matcher:
        errors.append(".opencode/lib/heavy-command.mjs: a second matcher exists")
    if "--self-check" not in fast_text or POLICY not in fast_text:
        errors.append("scripts/verify-fast.sh: missing the policy self-check")
    if "check-agent-command-guard.py" not in fast_text:
        errors.append("scripts/verify-fast.sh: missing this check")
    return errors


def load(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise SystemExit(f"check-agent-command-guard: cannot read {path}: {error}")
    if not isinstance(document, dict):
        raise SystemExit(f"check-agent-command-guard: {path} is not a JSON object")
    return document


def self_check() -> int:
    claude = {
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        {
                            "type": "command",
                            "command": 'python3 "${CLAUDE_PROJECT_DIR}/scripts/heavy_command_policy.py"',
                            "timeout": 10,
                            "onFailure": "block",
                        }
                    ],
                }
            ]
        }
    }
    codex = {
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        {
                            "type": "command",
                            "command": 'bash -c \'r=$(git rev-parse --show-toplevel) || r=$(git rev-parse --path-format=absolute --git-common-dir); exec python3 "$r/scripts/heavy_command_policy.py"\'',
                            "timeout": 10,
                        }
                    ],
                }
            ]
        }
    }
    plugin = "spawnSync('python3', [policy]) // heavy_command_policy.py"
    fast = (
        "start_check heavy-policy python3 scripts/heavy_command_policy.py --self-check\n"
        "start_check agent-guard python3 scripts/check-agent-command-guard.py"
    )
    assert validate(claude, codex, plugin, fast, False) == []

    def fails(mutated_claude, mutated_codex, mutated_plugin=plugin, mutated_fast=fast, legacy=False):
        assert validate(mutated_claude, mutated_codex, mutated_plugin, mutated_fast, legacy), "expected an error"

    broken = json.loads(json.dumps(claude))
    broken["hooks"]["PreToolUse"][0]["matcher"] = "bash"
    fails(broken, codex)
    broken = json.loads(json.dumps(claude))
    del broken["hooks"]["PreToolUse"][0]["hooks"][0]["onFailure"]
    fails(broken, codex)
    broken = json.loads(json.dumps(claude))
    broken["hooks"]["PreToolUse"][0]["hooks"][0]["timeout"] = 600
    fails(broken, codex)
    broken = json.loads(json.dumps(codex))
    broken["hooks"]["PreToolUse"][0]["hooks"][0]["command"] = "python3 scripts/heavy_command_policy.py"
    fails(claude, broken)
    broken = json.loads(json.dumps(codex))
    broken["hooks"]["PreToolUse"][0]["hooks"][0]["command"] = "python3 \"$(git rev-parse --show-toplevel)/scripts/heavy_command_policy.py\""
    fails(claude, broken)
    fails(claude, codex, "no policy here")
    fails(claude, codex, legacy=True)
    fails(claude, codex, mutated_fast="start_check cpu-budget python3 scripts/cpu-budget.py --self-check")
    print("check-agent-command-guard: self-check OK")
    return 0


def main() -> int:
    if "--self-check" in sys.argv[1:]:
        return self_check()
    claude = load(ROOT / ".claude" / "settings.json")
    codex = load(ROOT / ".codex" / "hooks.json")
    plugin_text = (ROOT / ".opencode" / "plugins" / "cpu-budget.ts").read_text(encoding="utf-8")
    fast_text = (ROOT / "scripts" / "verify-fast.sh").read_text(encoding="utf-8")
    errors = validate(
        claude,
        codex,
        plugin_text,
        fast_text,
        (ROOT / ".opencode" / "lib" / "heavy-command.mjs").exists(),
    )
    for error in errors:
        print(f"FAIL: {error}")
    if errors:
        return 1
    print("check-agent-command-guard: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
