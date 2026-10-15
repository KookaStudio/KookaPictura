#!/usr/bin/env python3
"""Verify the icon set is fully mapped and attributed.

Fails when:
- a file under assets/icons/ has no entry in assets/icons/lucide-map.json,
- a map entry has no file,
- a lucide/kind entry lacks a slug,
- LICENSES/Lucide.txt is missing.

Run from the repository root: python3 scripts/check-icon-provenance.py
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "assets" / "icons"
MAP = ICONS / "lucide-map.json"
LICENSE = ROOT / "LICENSES" / "Lucide.txt"


def main() -> int:
    errors: list[str] = []

    if not LICENSE.is_file():
        errors.append(f"missing {LICENSE.relative_to(ROOT)}")

    if not MAP.is_file():
        errors.append(f"missing {MAP.relative_to(ROOT)}")
        print("\n".join(errors))
        return 1

    doc = json.loads(MAP.read_text(encoding="utf-8"))
    entries: dict[str, dict] = doc.get("entries", {})
    files = {p.stem for p in ICONS.glob("*.svg")}

    for pid in sorted(files - set(entries)):
        errors.append(f"icon with no map entry: {pid}")
    for pid in sorted(set(entries) - files):
        errors.append(f"map entry with no icon file: {pid}")
    for pid, rec in sorted(entries.items()):
        if rec.get("kind") == "lucide" and not rec.get("slug"):
            errors.append(f"lucide entry without a slug: {pid}")
        if rec.get("kind") not in {"lucide", "custom", "brand", "note", "out-of-scope"}:
            errors.append(f"unknown kind for {pid}: {rec.get('kind')!r}")

    # The Lucide style contract for every vendored or hand-authored asset.
    import re

    root_re = re.compile(r"<svg\b[^>]*>", re.S)
    for pid, rec in sorted(entries.items()):
        if rec.get("kind") not in ("lucide", "custom", "brand"):
            continue
        svg = (ICONS / f"{pid}.svg").read_text(encoding="utf-8")
        root = root_re.search(svg)
        if not root:
            errors.append(f"{pid}: no <svg> root element")
            continue
        tag = root.group(0)
        # Brand marks (simple-icons) are filled silhouettes; everything else
        # follows the stroked Lucide contract.
        if rec["kind"] == "brand":
            wants = (("viewBox", "0 0 24 24"), ("fill", "currentColor"))
        else:
            wants = (("viewBox", "0 0 24 24"), ("fill", "none"),
                     ("stroke", "currentColor"), ("stroke-width", "2"))
        for attr, want in wants:
            if f'{attr}="{want}"' not in tag:
                errors.append(f'{pid}: root missing {attr}="{want}"')
        if re.search(r'(?:stroke|fill)="#', svg):
            errors.append(f"{pid}: hard-coded stroke/fill colour remains")

    if errors:
        print("icon provenance: FAILED")
        for e in errors:
            print(f"  {e}")
        return 1

    kind_counts: dict[str, int] = {}
    for rec in entries.values():
        kind_counts[rec["kind"]] = kind_counts.get(rec["kind"], 0) + 1
    summary = ", ".join(f"{k}={v}" for k, v in sorted(kind_counts.items()))
    print(f"icon provenance: OK ({len(entries)} icons; {summary})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
