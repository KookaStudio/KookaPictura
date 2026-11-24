#!/usr/bin/env python3
"""The changed path set shared by the local verification checks.

A path is changed when it differs from the merge base with the default branch
(`master`, `main`, `kooka/master`, `kooka/main`, or their `origin/` forms) or is
present in `git status --porcelain --untracked-files=all`. That is the set
`scripts/verify-fast.sh` already used for its docs-only fast path.

An empty set means "no usable change set" (clean tree above the merge base, or a
git failure), and callers then scan the whole tree instead of skipping their
check: skipping is how a gate goes quiet.

`scoped_paths(selector, changed)` is the library entry point: it reads a
`--changed` list and returns the entries the check cares about, or `None` when
the check should scan the whole tree. The CLI writes the list NUL-separated, so a
name holding a newline survives; the shell check reads it the same way.

Run from the repository root: python3 scripts/changed_files.py [--output FILE] [--self-test]
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from typing import Callable, Iterable

ROOT = Path(__file__).resolve().parent.parent
REFS = ("master", "main", "kooka/master", "kooka/main", "origin/master", "origin/main")


def git(root: Path, *args: str) -> str:
    result = subprocess.run(["git", *args], cwd=root, capture_output=True, check=False)
    # Decode with surrogateescape: a name holding a non-UTF-8 byte survives
    # instead of raising, and the checks stat it as the same byte string.
    return result.stdout.decode(errors="surrogateescape")


def merge_base(root: Path = ROOT, refs: Iterable[str] = REFS) -> str:
    for ref in refs:
        if not git(root, "rev-parse", "--verify", "-q", ref).strip():
            continue
        base = git(root, "merge-base", "HEAD", ref).strip()
        if base:
            return base
    return ""


def _porcelain_paths(text: str) -> list[str]:
    """Paths from `git status --porcelain -z`.

    `-z` never quotes a path and starts each record with `XY `, so a path with a
    space, a quote or a non-ASCII byte survives intact (the non-`-z` form quotes
    them, and a quoted path would match no file). A rename or copy record is
    followed by its original path as a second NUL-separated field.
    """
    fields = text.split("\0")
    paths: list[str] = []
    i = 0
    while i < len(fields):
        field = fields[i]
        i += 1
        if len(field) < 4:
            continue
        paths.append(field[3:])
        if field[0] in "RC" and i < len(fields):
            paths.append(fields[i])
            i += 1
    return paths


def changed_paths(
    root: Path = ROOT,
    base: str | None = None,
    refs: Iterable[str] = REFS,
) -> list[str]:
    """Repository-relative changed paths: the merge-base diff plus the work tree."""
    if base is None:
        base = merge_base(root, refs)
    paths: set[str] = set()
    if base:
        paths.update(git(root, "diff", "--name-only", "-z", base, "--").split("\0"))
    paths.update(_porcelain_paths(git(root, "status", "--porcelain", "-z", "--untracked-files=all")))
    return sorted(p for p in paths if p)


def read_changed_list(path: str) -> list[str]:
    """Read a `--changed` list; a missing file is a usage error, not a silent full scan."""
    try:
        data = Path(path).read_bytes()
    except OSError as error:
        print(f"changed_files: cannot read the changed list {path}: {error}", file=sys.stderr)
        raise SystemExit(2)
    # NUL-separated, so a name holding a newline survives; a newline-separated
    # list is still accepted for a hand-written file.
    if b"\0" in data:
        return [
            entry.decode(errors="surrogateescape")
            for entry in data.split(b"\0")
            if entry
        ]
    return [line for line in data.decode(errors="surrogateescape").splitlines() if line]


def scoped_paths(
    selector: Callable[[str], bool],
    changed: str | None,
    affected_all: Callable[[str], bool] | None = None,
) -> list[str] | None:
    """The changed paths the check cares about, or None for a whole-tree scan.

    None means: scoping was not requested, the changed set is empty, or the
    change touches something that can affect files the set does not name (a
    check's own allowlist, for example). All three fall back to the whole tree
    so an incomplete set cannot silence the check.
    """
    if changed is None:
        return None
    paths = read_changed_list(changed)
    if not paths:
        return None
    if affected_all is not None and any(affected_all(path) for path in paths):
        return None
    return [path for path in paths if selector(path)]


def _self_test() -> int:
    import contextlib
    import io
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        git(root, "init", "-q", "-b", "master")
        git(root, "config", "user.email", "check@example.invalid")
        git(root, "config", "user.name", "check")
        (root / "a.rs").write_text("a\n", encoding="utf-8")
        (root / "b.md").write_text("b\n", encoding="utf-8")
        git(root, "add", ".")
        git(root, "commit", "-q", "-m", "base")
        base = git(root, "rev-parse", "HEAD").strip()

        (root / "a.rs").write_text("a\nb\n", encoding="utf-8")
        (root / "c.rs").write_text("c\n", encoding="utf-8")
        assert changed_paths(root, base=base) == ["a.rs", "c.rs"]
        assert [p for p in changed_paths(root, base=base) if p.endswith(".rs")] == ["a.rs", "c.rs"]
        assert [p for p in changed_paths(root, base=base) if p.endswith(".md")] == []
        assert merge_base(root, ("master",)) == base
        assert merge_base(root, ("no-such-ref",)) == ""
        (root / "c.rs").unlink()
        (root / "a.rs").write_text("a\n", encoding="utf-8")
        assert changed_paths(root, base=base) == []

        listing = root / "list.txt"
        listing.write_bytes(b"a.rs\0\0c.rs\0")
        assert read_changed_list(str(listing)) == ["a.rs", "c.rs"]
        listing.write_text("a.rs\nb.md\n", encoding="utf-8")
        assert read_changed_list(str(listing)) == ["a.rs", "b.md"]
        listing.write_bytes(b"a.rs\0c.rs\0")
        assert scoped_paths(lambda p: p.endswith(".rs"), str(listing)) == ["a.rs", "c.rs"]
        listing.write_text("a.rs\nb.md\n", encoding="utf-8")
        assert scoped_paths(lambda p: True, str(listing), lambda p: p == "b.md") is None
        listing.write_bytes(b"")
        assert scoped_paths(lambda p: True, str(listing)) is None
        assert scoped_paths(lambda p: True, None) is None
        try:
            with contextlib.redirect_stderr(io.StringIO()):
                scoped_paths(lambda p: True, str(root / "missing.txt"))
        except SystemExit as error:
            assert error.code == 2, error.code
        else:
            raise AssertionError("a missing changed list was accepted")

        # A path with a space or a non-ASCII byte must survive verbatim, and a
        # rename must contribute both names: a quoted or "old -> new" path
        # matches no file and would silently drop it from a scoped scan.
        (root / "with space.rs").write_text("s\n", encoding="utf-8")
        (root / "t\u00ebst.rs").write_text("t\n", encoding="utf-8")
        # A name that is not valid UTF-8 must round-trip, not raise.
        (root / "bad\udcffname.rs").write_bytes(b"b\n")
        git(root, "mv", "b.md", "renamed.md")
        got = changed_paths(root, base=base)
        for name in ("with space.rs", "t\u00ebst.rs", "bad\udcffname.rs", "renamed.md", "b.md"):
            assert name in got, f"{name!r} missing from {got!r}"
        assert not any(p.startswith('"') or " -> " in p for p in got), got
    print("changed_files: self-test OK")
    return 0


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return _self_test()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default=None, help="write the NUL-separated list to this file")
    args = parser.parse_args()
    paths = changed_paths()
    # Encode with surrogateescape so a non-UTF-8 byte round-trips unchanged.
    data = b"".join(path.encode(errors="surrogateescape") + b"\0" for path in paths)
    if args.output:
        Path(args.output).write_bytes(data)
    sys.stdout.buffer.write(data)
    return 0


if __name__ == "__main__":
    sys.exit(main())
