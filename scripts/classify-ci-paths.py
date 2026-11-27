#!/usr/bin/env python3
"""Classify a GitHub Actions changed-file list into CI coverage buckets."""
from __future__ import annotations

import argparse
from fnmatch import fnmatchcase
import json
import os
from pathlib import Path
import sys


BUCKETS = ("rust", "cpp", "render", "docs")
CPP_PREFIX = "crates/pictura-app/cpp/"
RENDER_PATTERNS = (
    "crates/pictura-render/**",
    "crates/pictura-core/**",
    "crates/pictura-codec/**",
    "crates/pictura-color/**",
    "crates/pictura-adjust/**",
    "crates/pictura-filters/**",
    "crates/pictura-ops/**",
    "crates/pictura-paint/**",
    "crates/pictura-select/**",
    "crates/pictura-testkit/**",
    "crates/pictura-app/src/gpu*",
    "crates/pictura-app/Cargo.toml",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/**",
    ".github/workflows/**",
)


def run_all() -> dict[str, bool]:
    return {"rust": True, "cpp": True, "render": True, "docs": False}


def is_docs(path: str) -> bool:
    return path.startswith(("docs/", "openspec/", ".serena/")) or path.endswith(".md")


def classify(paths: list[str]) -> dict[str, bool]:
    if not paths:
        return run_all()
    if all(is_docs(path) for path in paths):
        return {"rust": False, "cpp": False, "render": False, "docs": True}
    return {
        "rust": any(not path.startswith(CPP_PREFIX) for path in paths),
        "cpp": True,
        "render": any(
            fnmatchcase(path, pattern)
            for path in paths
            for pattern in RENDER_PATTERNS
        ),
        "docs": False,
    }


def parse_classification(raw: str, filter_ok: str) -> tuple[dict[str, bool], str | None]:
    if filter_ok.lower() != "true":
        return run_all(), "changed-file filter failed"
    try:
        paths = json.loads(raw)
    except json.JSONDecodeError as error:
        return run_all(), f"invalid changed-file JSON: {error}"
    if not isinstance(paths, list) or any(
        not isinstance(path, str) or not path for path in paths
    ):
        return run_all(), "changed-file output is not a list of non-empty paths"
    return classify(paths), None


def write_outputs(path: str, outputs: dict[str, bool]) -> None:
    with Path(path).open("a", encoding="utf-8") as handle:
        for bucket in BUCKETS:
            handle.write(f"{bucket}={str(outputs[bucket]).lower()}\n")


def self_check() -> None:
    docs_only = {"rust": False, "cpp": False, "render": False, "docs": True}
    cases = [
        (["crates/pictura-app/cpp/panel.cpp"], {
            "rust": False, "cpp": True, "render": False, "docs": False,
        }),
        (["crates/pictura-app/cpp/a file.cpp"], {
            "rust": False, "cpp": True, "render": False, "docs": False,
        }),
        (["crates/pictura-mcp/src/lib.rs"], {
            "rust": True, "cpp": True, "render": False, "docs": False,
        }),
        (["crates/pictura-script/src/lib.rs"], {
            "rust": True, "cpp": True, "render": False, "docs": False,
        }),
        (["crates/pictura-render/src/lib.rs"], {
            "rust": True, "cpp": True, "render": True, "docs": False,
        }),
        (["crates/pictura-app/src/gpu.rs"], {
            "rust": True, "cpp": True, "render": True, "docs": False,
        }),
        ([".github/workflows/ci.yml"], {
            "rust": True, "cpp": True, "render": True, "docs": False,
        }),
        (["README.md"], docs_only),
        (["docs/guide.md", "openspec/config.yaml"], docs_only),
        (["crates/pictura-app/cpp/panel.cpp", "crates/pictura-render/src/lib.rs"], {
            "rust": True, "cpp": True, "render": True, "docs": False,
        }),
        (["unfamiliar-root-file"], {
            "rust": True, "cpp": True, "render": False, "docs": False,
        }),
        ([], run_all()),
    ]
    for paths, expected in cases:
        assert classify(paths) == expected, paths

    outputs, error = parse_classification("[]", "true")
    assert error is None and outputs == run_all()
    for raw, filter_ok in (
        ("not JSON", "true"),
        ("{}", "true"),
        ("[1]", "true"),
        ("[]", "false"),
    ):
        outputs, error = parse_classification(raw, filter_ok)
        assert error is not None and outputs == run_all(), (raw, filter_ok)
    print("ci-paths: self-check OK")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--files", default=os.environ.get("FILES", ""))
    parser.add_argument("--filter-ok", default=os.environ.get("FILTER_OK", "false"))
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()
    if args.self_check:
        self_check()
        return 0

    outputs, error = parse_classification(args.files, args.filter_ok)
    output_path = os.environ.get("GITHUB_OUTPUT")
    if output_path:
        write_outputs(output_path, outputs)
    if error:
        print(f"ci-paths: FAILED: {error}; running all build buckets", file=sys.stderr)
        return 1
    print("ci-paths: " + ", ".join(f"{bucket}={str(outputs[bucket]).lower()}" for bucket in BUCKETS))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
