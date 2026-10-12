#!/usr/bin/env python3
"""Generate THIRD-PARTY-LICENSES from the resolved Cargo dependency graph.

Run from the repository root:  python3 scripts/third-party-licenses.py

The file is generated, not hand-edited. It lists every non-workspace crate in
the lockfile with its declared license so the shipped tree's obligations are
auditable; the full texts of the licenses that require reproduction live under
LICENSES/ and in each crate's upstream.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def cargo_metadata() -> dict:
    out = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=ROOT,
    )
    return json.loads(out)


def reachable(meta: dict) -> set[str]:
    nodes = {n["id"]: n for n in meta.get("resolve", {}).get("nodes", [])}
    seen: set[str] = set()
    stack = list(meta["workspace_members"])
    while stack:
        node_id = stack.pop()
        if node_id in seen:
            continue
        seen.add(node_id)
        for dep in nodes.get(node_id, {}).get("deps", []):
            stack.append(dep["pkg"])
    return seen


def main() -> None:
    meta = cargo_metadata()
    workspace = set(meta["workspace_members"])
    ids = reachable(meta)
    packages = {p["id"]: p for p in meta["packages"]}

    rows = sorted(
        (
            p["name"],
            p["version"],
            p.get("license") or "UNKNOWN — inspect upstream",
        )
        for pid, p in packages.items()
        if pid in ids and pid not in workspace
    )

    lines = [
        "THIRD-PARTY LICENSES",
        "====================",
        "",
        "Kooka Pictura is licensed under the GNU General Public License v3.0 or",
        "later (see LICENSE). It links the Qt 6 framework and the open-source Rust",
        "crates listed below.",
        "",
        "Qt 6 (Core, Gui, Widgets, Svg, Network) — LGPL-3.0-or-later (also available",
        "under GPL-3.0 or a commercial license). Kooka Pictura dynamically links",
        "system Qt; the corresponding source is offered upstream at",
        "https://code.qt.io/ . Required license texts are under LICENSES/.",
        "",
        f"Rust dependencies in the resolved graph: {len(rows)}",
        "",
        "| Crate | Version | License |",
        "|---|---|---|",
    ]
    lines += (
        f"| `{name}` | {version} | {license_} |" for name, version, license_ in rows
    )
    lines += [
        "",
        "Icon assets",
        "-----------",
        "",
        "The icons in assets/icons/ are derived from the Lucide icon set (vendored",
        "from lucide-static 1.50.0, https://lucide.dev) or are independently",
        "authored in the same style. Lucide is licensed under the ISC License,",
        "with a subset derived from the Feather project under the MIT License.",
        "Full text in LICENSES/Lucide.txt; per-icon provenance in",
        "docs/dev/icon-provenance.md.",
        "",
        "Ported code",
        "-----------",
        "",
        "Filter > Liquify (crates/pictura-filters/src/distort/liquify.rs and",
        "its submodules) and Filter > Artistic > Palette Knife are ported from",
        "PhotoCraft (https://github.com/storytold/photocraft), Copyright (c) 2026",
        "ArtCraft Team and the PhotoCraft contributors, under the MIT License",
        "(PhotoCraft is MIT OR Apache-2.0). Full text in LICENSES/photocraft.txt.",
        "",
        "Font assets",
        "------------",
        "",
        "The UI font is Noto Sans (assets/fonts/, vendored from",
        "notofonts/noto-fonts, the Google Fonts static-instance source).",
        "Liberation Sans is bundled by pictura-render as the text fallback.",
        "Both are licensed under the SIL Open Font License 1.1; full texts in",
        "LICENSES/NotoSans-OFL.txt and",
        "crates/pictura-render/assets/LICENSES/LiberationSans-OFL.txt.",
        "",
        "Full license texts for components that require reproduction are kept in",
        "LICENSES/. Regenerate this file with scripts/third-party-licenses.py; do",
        "not edit it by hand.",
        "",
    ]

    (ROOT / "THIRD-PARTY-LICENSES").write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote THIRD-PARTY-LICENSES ({len(rows)} packages)")


if __name__ == "__main__":
    main()
