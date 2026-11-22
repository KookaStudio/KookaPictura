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
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Optional

ROOT = Path(__file__).resolve().parent.parent
POLICY = "heavy_command_policy.py"


def test_managed_policy_segments() -> None:
    policy = ROOT / "scripts" / POLICY
    runner = f"python3 {shlex.quote(str(ROOT / 'scripts' / 'agent-build.py'))} --"
    for command, refused in (
        (f"{runner} cargo build", False),
        (f"{runner} bash -lc 'cargo build && ninja'", False),
        (f"{runner} true && cargo build", True),
        (f"cargo build; {runner} true", True),
        ("echo scripts/with-build-lock.sh && cargo build", True),
        ("bash scripts/with-build-lock.sh -- cargo build", True),
        ("python3 scripts/cpu-budget.py exec -- cargo test", True),
        ("bash -- scripts/verify-full.sh", True),
        ("setsid nohup env PICTURA_VERIFY_LOCK=0 bash scripts/verify-full.sh", True),
        ("PICTURA_VERIFY_LOCK_HELD=1 timeout 60 /usr/bin/cmake --build build", True),
        ("bash -lc 'git status && /usr/bin/ninja -C build'", True),
        ("bash -euo pipefail -c 'ninja -C build'", True),
        ("env -S 'PICTURA_VERIFY_LOCK=0 /usr/bin/ninja -C build'", True),
        ("eval 'cargo build'", True),
        ("(cargo test)", True),
        ("echo 'cargo build; ninja -C build' && cargo fmt --all", False),
    ):
        result = subprocess.run(
            [sys.executable, str(policy), "--managed", "--command", command],
            text=True, capture_output=True, check=False, timeout=5,
        )
        assert result.returncode == (2 if refused else 0), (command, result)
        if refused:
            assert "agent-build.py" in result.stderr, result.stderr


def test_opencode_policy_failure() -> None:
    javascript = """
import assert from "node:assert/strict"
import childProcess from "node:child_process"
import { syncBuiltinESMExports } from "node:module"
import { join } from "node:path"
import { pathToFileURL } from "node:url"

const [fixture, stale] = process.argv.slice(2)
const outcomes = {
  "allow.py": { status: 0 },
  "deny.py": { status: 2, stderr: "refused by fixture" },
  "unexpected.py": { status: 42 },
  "timeout.py": { error: new Error("fixture timed out") },
}
childProcess.spawnSync = (program, [policy, ...args], options) => {
  assert.equal(program, "python3")
  assert.deepEqual(args, ["--managed", "--command", "true"])
  assert.equal(options.timeout, 5000)
  const name = Object.keys(outcomes).find((name) => policy === join(fixture, name))
  assert.ok(name, policy)
  return outcomes[name]
}
syncBuiltinESMExports()
const { CpuBudget } = await import(pathToFileURL(join(fixture, "guard.mjs")).href)
const [tool, args] = [{ tool: "bash" }, { args: { command: "true" } }]
const hook = async (name) => {
  process.env.PICTURA_AGENT_POLICY = join(fixture, name)
  const plugin = await CpuBudget({ directory: stale, worktree: stale })
  return plugin["tool.execute.before"](tool, args)
}
await hook("allow.py")
await assert.rejects(() => hook("deny.py"), /refused by fixture/)
await assert.rejects(() => hook("missing.py"), /missing.py/)
await assert.rejects(() => hook("unexpected.py"), /status 42/)
await assert.rejects(() => hook("timeout.py"), /fixture timed out/)
"""
    with tempfile.TemporaryDirectory() as temp:
        path = Path(temp)
        (path / "guard.mjs").write_bytes((ROOT / ".opencode/plugins/cpu-budget.ts").read_bytes())
        for policy in ("allow.py", "deny.py", "unexpected.py", "timeout.py"):
            (path / policy).touch()
        stale = path / "stale"
        (stale / "scripts").mkdir(parents=True)
        (stale / "scripts" / POLICY).touch()
        subprocess.run(
            ["node", "--input-type=module", "-", str(path), str(stale)],
            input=javascript, text=True, check=True, timeout=12,
        )


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
    test_opencode_policy_failure()
    test_managed_policy_segments()
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
    test_opencode_policy_failure()
    test_managed_policy_segments()
    print("check-agent-command-guard: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
