#!/usr/bin/env python3
"""Load, validate, select, and run the repository gate manifest."""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "scripts" / "checks.toml"
PLACEMENTS = ("local_fast", "local_full", "docs_only", "ci_guards", "ci_build")
CLI_PLACEMENTS = tuple(name.replace("_", "-") for name in PLACEMENTS)
REQUIRED_FIELDS = {
    "id", "scope", "cost", "phase", "command", "local_fast", "local_full", "ci_guards"
}
OPTIONAL_FIELDS = {"docs_only", "ci_build", "script", "scripts"}
MANDATORY_CI = frozenset({
    "agent-build", "agent-command-guard", "agent-cpu-budget", "archive",
    "build-caching", "build-lock", "cpu-budget", "file-size", "flatpak", "gate-manifest",
    "guard", "heavy-policy", "icon-provenance", "openspec", "parity",
    "selftest-budget", "selftest-codes", "shell-seams", "source-fragments",
    "test-plan", "worktree-hook",
})
BUILD_CI_EQUIVALENTS = frozenset({"build-tree", "clippy", "fmt", "tests", "view-oracle"})


class ManifestError(ValueError):
    pass


def load(path: Path = MANIFEST) -> list[dict]:
    with path.open("rb") as handle:
        rows = tomllib.load(handle).get("check")
    if not isinstance(rows, list) or not rows:
        raise ManifestError("checks.toml must contain at least one [[check]] table")
    return rows


def script_paths(rows: list[dict]) -> list[str]:
    paths = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        scripts = row.get("scripts", [])
        if isinstance(scripts, list):
            paths.extend(item for item in scripts if isinstance(item, str))
        script = row.get("script")
        if isinstance(script, str):
            paths.append(script)
    return sorted(paths)


def validate(rows: list[dict], root: Path = ROOT) -> list[str]:
    errors = []
    ids = set()
    seen_scripts = set()
    for index, row in enumerate(rows):
        label = f"check[{index}]"
        if not isinstance(row, dict):
            errors.append(f"{label}: expected a table")
            continue
        missing = REQUIRED_FIELDS - row.keys()
        unknown = row.keys() - REQUIRED_FIELDS - OPTIONAL_FIELDS
        if missing:
            errors.append(f"{label}: missing fields {sorted(missing)}")
        if unknown:
            errors.append(f"{label}: unknown fields {sorted(unknown)}")
        check_id = row.get("id")
        if not isinstance(check_id, str) or not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", check_id):
            errors.append(f"{label}: invalid id {check_id!r}")
        elif check_id in ids:
            errors.append(f"{label}: duplicate id {check_id}")
        else:
            ids.add(check_id)
        if row.get("scope") not in ("guards", "build", "manual"):
            errors.append(f"{label}: invalid scope {row.get('scope')!r}")
        if row.get("cost") not in ("cheap", "moderate", "expensive"):
            errors.append(f"{label}: invalid cost {row.get('cost')!r}")
        if row.get("phase") not in ("sequential", "parallel"):
            errors.append(f"{label}: invalid phase {row.get('phase')!r}")
        command = row.get("command")
        if not isinstance(command, str) or not command.strip():
            errors.append(f"{label}: command must be a non-empty string")
            command = ""
        for field in ("local_fast", "local_full", "ci_guards", "docs_only", "ci_build"):
            if field in row and not isinstance(row[field], bool):
                errors.append(f"{label}: {field} must be boolean")
        scripts = row.get("scripts", [])
        if not isinstance(scripts, list) or any(not isinstance(item, str) for item in scripts):
            errors.append(f"{label}: scripts must be a string array")
            scripts = []
        script = row.get("script")
        if script is not None and not isinstance(script, str):
            errors.append(f"{label}: script must be a string")
            script = None
        for path in [*scripts, *([script] if script else [])]:
            if path in seen_scripts:
                errors.append(f"{label}: duplicate script path {path}")
            seen_scripts.add(path)
            if not path.startswith("scripts/") or ".." in Path(path).parts:
                errors.append(f"{label}: invalid script path {path!r}")
            elif not (root / path).is_file():
                errors.append(f"{label}: missing script {path}")
        if script and script not in command:
            errors.append(f"{label}: command does not invoke {script}")
        local = row.get("local_fast") or row.get("local_full")
        if local and not (row.get("ci_guards") or row.get("ci_build")):
            errors.append(f"{label}: local-only check needs ci_guards or ci_build")
        if row.get("local_fast") and not row.get("local_full"):
            errors.append(f"{label}: local_fast requires local_full")
        if row.get("ci_guards") and not row.get("local_full"):
            errors.append(f"{label}: ci_guards requires local_full")
        if row.get("docs_only") and not all(row.get(key) for key in ("local_fast", "local_full", "ci_guards")):
            errors.append(f"{label}: docs_only requires all guard placements")
        if row.get("ci_build") and row.get("scope") != "build":
            errors.append(f"{label}: ci_build requires build scope")
        if row.get("ci_build") and (
            not isinstance(check_id, str) or check_id not in BUILD_CI_EQUIVALENTS
        ):
            errors.append(f"{label}: unknown build-CI equivalent {check_id!r}")
        if row.get("scope") == "guards" and not row.get("ci_guards"):
            errors.append(f"{label}: guards scope requires ci_guards")
        if row.get("scope") == "manual" and any(row.get(key) for key in PLACEMENTS):
            errors.append(f"{label}: manual scope cannot have automatic placement")
    ordered_ids = [row.get("id") if isinstance(row, dict) else "" for row in rows]
    if ordered_ids != sorted(ordered_ids, key=str):
        errors.append("check IDs are not sorted")
    ci_ids = {
        row.get("id") for row in rows
        if isinstance(row, dict) and isinstance(row.get("id"), str) and row.get("ci_guards") is True
    }
    missing_ci = sorted(MANDATORY_CI - ci_ids)
    if missing_ci:
        errors.append(f"missing mandatory CI IDs: {missing_ci}")
    return errors


def select(
    rows: list[dict],
    placement: str,
    *,
    phase: str = "all",
    full_local: bool = False,
) -> list[dict]:
    if placement not in PLACEMENTS:
        raise ManifestError(f"unknown placement {placement}")
    if placement == "local_full" and not full_local:
        raise ManifestError("local_full requires PICTURA_FULL_GATES=1")
    return [
        row for row in rows
        if row.get(placement)
        and (phase == "all" or row["phase"] == phase)
    ]


def deferred(rows: list[dict]) -> list[dict]:
    return [row for row in rows if row.get("local_full") and not row.get("local_fast")]


def announce_deferred(rows: list[dict]) -> None:
    for row in deferred(rows):
        print(f"deferred {row['id']}: enable PICTURA_FULL_GATES=1", file=sys.stderr)


def run_one(check: dict, root: Path, env: dict[str, str]) -> tuple[str, int, str]:
    print(f"start {check['id']}", flush=True)
    result = subprocess.run(
        ["bash", "-e", "-c", check["command"]],
        cwd=root,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        errors="replace",
        check=False,
    )
    return check["id"], result.returncode, result.stdout


def report(results: list[tuple[str, int, str]]) -> int:
    failed = 0
    for check_id, returncode, output in results:
        if returncode == 0:
            if check_id == "tests" and output:
                print(output, end="" if output.endswith("\n") else "\n")
            print(f"{check_id}=ok")
            continue
        failed = 1
        print(f"{check_id}=FAILED (rc={returncode})")
        for line in output.splitlines():
            print(f"  {line}")
    return failed


def execute(
    checks: list[dict],
    *,
    root: Path = ROOT,
    placement: str,
    changed_file: str = "",
    jobs: int = 1,
    stop_on_failure: bool = False,
) -> int:
    if jobs < 1:
        raise ManifestError("parallel worker count must be positive")
    env = os.environ.copy()
    env["GATE_PLACEMENT"] = placement
    env["GATE_CHANGED_FILE"] = changed_file
    if jobs == 1 or stop_on_failure:
        results = []
        for check in checks:
            result = run_one(check, root, env)
            results.append(result)
            if stop_on_failure and result[1] != 0:
                break
    else:
        with ThreadPoolExecutor(max_workers=jobs) as pool:
            results = list(pool.map(lambda check: run_one(check, root, env), checks))
    return report(results)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("run",))
    parser.add_argument("run", choices=CLI_PLACEMENTS)
    parser.add_argument("--phase", choices=("all", "sequential", "parallel"), default="all")
    parser.add_argument("--changed")
    parser.add_argument("--parallel", type=int, default=1)
    args = parser.parse_args()
    full_value = os.environ.get("PICTURA_FULL_GATES", "0")
    if full_value not in {"0", "1"}:
        parser.error("PICTURA_FULL_GATES must be 0 or 1")
    try:
        rows = load()
        errors = validate(rows)
        if errors:
            raise ManifestError("\n".join(errors))
        if args.changed and not Path(args.changed).is_file():
            raise ManifestError(f"changed-path list is not readable: {args.changed}")
        field = args.run.replace("-", "_")
        checks = select(
            rows, field, phase=args.phase, full_local=full_value == "1"
        )
        if field == "local_fast":
            announce_deferred(rows)
        stop = field in {"ci_guards", "docs_only"} or args.phase == "sequential"
        return execute(
            checks,
            placement=args.run,
            changed_file=args.changed or "",
            jobs=args.parallel,
            stop_on_failure=stop,
        )
    except (ManifestError, OSError, tomllib.TOMLDecodeError) as error:
        print(f"gate manifest: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
