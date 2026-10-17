#!/usr/bin/env python3
"""Check that flatpak/cargo-sources.json covers Cargo.lock.

The Flatpak build is offline and resolves every registry crate from the
committed cargo-sources.json. Nothing regenerates that file automatically, so a
lockfile bump can silently drift. This check is structural (names and versions);
it does not verify checksums. Regenerate with:

    flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json

Run from the repository root: python3 scripts/check-flatpak-cargo-sources.py
"""
from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCK = ROOT / "Cargo.lock"
SOURCES = ROOT / "flatpak" / "cargo-sources.json"


def binary_versions(text: str) -> set[tuple[str, str]]:
    data = json.loads(text)
    found = set()
    for entry in data:
        url = entry.get("url") or ""
        if not url.endswith(".crate"):
            continue
        name = url.rsplit("/", 2)[-2]
        filename = url.rsplit("/", 1)[-1]
        prefix = name + "-"
        if filename.startswith(prefix):
            found.add((name, filename[len(prefix) : -len(".crate")]))
    return found


def registry_crates(text: str) -> set[tuple[str, str]]:
    data = tomllib.loads(text)
    return {
        (p["name"], p["version"])
        for p in data.get("package", [])
        if p.get("source", "").startswith("registry+")
    }


def missing(lock_text: str, sources_text: str) -> list[tuple[str, str]]:
    return sorted(registry_crates(lock_text) - binary_versions(sources_text))


def self_test() -> int:
    lock = """
[[package]]
name = "adler2"
version = "2.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "kooka-pictura"
version = "0.1.0"
"""
    sources = json.dumps(
        [
            {
                "type": "archive",
                "url": "https://static.crates.io/crates/adler2/adler2-2.0.1.crate",
                "sha256": "00",
            },
            {"type": "inline", "contents": ""},
        ]
    )
    assert registry_crates(lock) == {("adler2", "2.0.1")}
    assert binary_versions(sources) == {("adler2", "2.0.1")}
    assert missing(lock, sources) == []
    stale = json.dumps(
        [
            {
                "type": "archive",
                "url": "https://static.crates.io/crates/adler2/adler2-2.0.0.crate",
                "sha256": "00",
            }
        ]
    )
    assert missing(lock, stale) == [("adler2", "2.0.1")]
    print("check-flatpak-cargo-sources: self-test OK")
    return 0


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test()
    absent = missing(
        LOCK.read_text(encoding="utf-8"), SOURCES.read_text(encoding="utf-8")
    )
    if absent:
        for name, version in absent:
            print(f"FAIL: {name} {version} is in Cargo.lock but not {SOURCES.name}")
        print("Regenerate flatpak/cargo-sources.json with flatpak-cargo-generator.py")
        return 1
    print("check-flatpak-cargo-sources: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
