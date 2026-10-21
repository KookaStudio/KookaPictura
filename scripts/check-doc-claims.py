#!/usr/bin/env python3
"""Keep the user-facing documentation honest about what the tree implements.

Three checks, all mechanical:

1. **The corpus may not deny the implementation.** No file under `docs/` may
   assert that the repository contains no source code, no build files, or no
   dependencies. The check is wrap-aware: 26 of the corpus banners hide the
   claim behind a line break, so blockquote markers are stripped and
   whitespace normalised before matching.
2. **Every open roadmap item names the issue that tracks it.** An unticked
   `ROADMAP.md` item without a `#NNN` reference is a gap nobody is tracking.
3. **Every path the roadmap and the README cite exists.** A claim that points
   at a file that is not there is not checkable, so it is not a claim.

`docs/dev/` is excluded from (1): it holds working notes that may quote the
old claim, and it is not the behavioural contract.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / "docs"
ROADMAP = ROOT / "ROADMAP.md"
README = ROOT / "README.md"

# The claim the corpus must not make about the repository itself.
DENIAL_PATTERNS = [
    re.compile(r"no\s+code\s+exists", re.I),
    re.compile(r"contains\s+\**no\s+source\b", re.I),
    re.compile(r"contains\s+\**no\s+code\b", re.I),
    re.compile(r"no\s+code\s*,\s*no\s+build\s+files", re.I),
    re.compile(r"no\s+source\s*,\s*build\s+files", re.I),
]

OPEN_ITEM = re.compile(r"^\s*-\s*\[ \]\s*(?P<body>.*)$")
ISSUE_REF = re.compile(r"#\d+")
BACKTICK = re.compile(r"`([^`\n]+)`")
LINE_SUFFIX = re.compile(r":\d+(-\d+)?$")

# A cited path is a backticked token that names a top-level entry of the
# repository. Anything else (a Rust module path, a URL, a crate name) is prose.
TOP_LEVEL_DIRS = {
    ".github", "assets", "crates", "docs", "flatpak", "openspec", "packaging",
    "reference-fixtures", "scripts", "site", "snap",
}
TOP_LEVEL_FILES = {
    "AGENTS.md", "CHANGELOG.md", "CMakeLists.txt", "CONTRIBUTING.md",
    "Cargo.toml", "DEVELOPING.md", "LICENSE", "NOTICE.md", "README.md",
    "ROADMAP.md", "deny.toml", "rust-toolchain.toml", "version.txt",
}


def corpus_files() -> list[Path]:
    return sorted(
        p for p in CORPUS.rglob("*.md")
        if "dev" not in p.relative_to(CORPUS).parts
    )


def flatten(text: str) -> str:
    """Drop blockquote markers and collapse whitespace so a wrapped claim matches."""
    unquoted = re.sub(r"^\s*>\s?", "", text, flags=re.M)
    return re.sub(r"\s+", " ", unquoted)


def corpus_denials() -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    for path in corpus_files():
        flat = flatten(path.read_text(encoding="utf-8", errors="replace"))
        for pattern in DENIAL_PATTERNS:
            match = pattern.search(flat)
            if match:
                rel = path.relative_to(ROOT).as_posix()
                start = max(0, match.start() - 40)
                found.append((rel, flat[start:match.end() + 40].strip()))
                break
    return found


def roadmap_open_items_without_issue() -> list[tuple[int, str]]:
    if not ROADMAP.is_file():
        return []
    found: list[tuple[int, str]] = []
    lines = ROADMAP.read_text(encoding="utf-8").splitlines()
    for index, line in enumerate(lines):
        match = OPEN_ITEM.match(line)
        if not match:
            continue
        body = match.group("body")
        # A wrapped item continues on the indented lines that follow it.
        for follow in lines[index + 1:]:
            if not follow.strip() or follow[:1] not in (" ", "\t"):
                break
            body += " " + follow.strip()
        if not ISSUE_REF.search(body):
            found.append((index + 1, line.strip()))
    return found


def cited_paths(path: Path) -> list[tuple[str, str]]:
    if not path.is_file():
        return []
    found: list[tuple[str, str]] = []
    for token in BACKTICK.findall(path.read_text(encoding="utf-8")):
        candidate = LINE_SUFFIX.sub("", token.strip())
        if "/" in candidate:
            head = candidate.split("/", 1)[0]
            if head not in TOP_LEVEL_DIRS:
                continue
        elif candidate not in TOP_LEVEL_FILES:
            continue
        if not (ROOT / candidate).exists():
            found.append((path.relative_to(ROOT).as_posix(), candidate))
    return found


def main() -> int:
    groups = sys.argv[1:] or ["corpus", "roadmap", "paths"]
    unknown = [g for g in groups if g not in ("corpus", "roadmap", "paths")]
    if unknown:
        print(f"check-doc-claims: unknown group {unknown}; use corpus, roadmap, paths")
        return 2
    failed = False

    if "corpus" in groups:
        denials = corpus_denials()
        if denials:
            print("check-doc-claims: FAILED (the corpus denies the implementation)")
            for rel, snippet in denials:
                print(f"  {rel}: ...{snippet}...")
            print("The corpus is the behavioural contract for code that exists; say where the")
            print("code is and point at docs/dev/doc-code-parity-gaps.md for what shipped.")
            failed = True

    if "roadmap" in groups:
        untracked = roadmap_open_items_without_issue()
        if untracked:
            print("check-doc-claims: FAILED (an open ROADMAP.md item names no tracking issue)")
            for number, line in untracked:
                print(f"  ROADMAP.md:{number}: {line}")
            print("Every open item must link the GitHub issue that tracks the gap.")
            failed = True

    if "paths" in groups:
        missing = cited_paths(ROADMAP) + cited_paths(README)
        if missing:
            print("check-doc-claims: FAILED (a cited path does not exist)")
            for rel, candidate in missing:
                print(f"  {rel}: {candidate}")
            print("A claim that points at a file that is not there is not checkable.")
            failed = True

    if failed:
        return 1
    print("check-doc-claims: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
