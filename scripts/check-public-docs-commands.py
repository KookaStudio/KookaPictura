#!/usr/bin/env python3
"""Verify the public user guide covers every menu command and UI reference.

The command catalogue is the source of truth. Every catalogue id with a menu
path must appear in the hidden manifest of its top-level menu page (or
``preferences.md`` for ``edit.preferences.*``). Every id without a path is a
panel- or tool-internal action and must be listed in
``scripts/public-docs-command-allowlist.txt`` with a reason.

The script also checks the View extra screenshots, the Preferences page names,
the status-bar readout names, and the guards-workflow wiring.

Run from the repository root: python3 scripts/check-public-docs-commands.py
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENTRIES = Path("crates/pictura-script/src/command_catalogue/entries.rs")
DOCS = Path("public_docs")
ALLOWLIST = Path("scripts/public-docs-command-allowlist.txt")
GUARDS = Path(".github/workflows/guards.yml")
VERIFY_FAST = Path("scripts/verify-fast.sh")

PAGE_FOR_TOP = {
    "File": "menu-file.md",
    "Edit": "menu-edit.md",
    "Image": "menu-image.md",
    "Layer": "menu-layer.md",
    "Type": "menu-type.md",
    "Select": "menu-select.md",
    "Filter": "menu-filter.md",
    "View": "menu-view.md",
    "Window": "menu-window.md",
    "Help": "menu-help.md",
}

VIEW_EXTRAS = {
    "view.show.selectionEdges": "view-show-selection-edges.png",
    "view.show.target.path": "view-show-target-path.png",
    "view.show.layer.edges": "view-show-layer-edges.png",
    "view.show.guides": "view-show-guides.png",
    "view.show.grid": "view-show-grid.png",
    "view.show.count": "view-show-count.png",
    "view.show.smartGuides": "view-show-smart-guides.png",
    "view.show.slices": "view-show-slices.png",
    "view.show.notes": "view-show-notes.png",
    "view.show.pixel.grid": "view-show-pixel-grid.png",
}

PREFERENCES_LABELS = [
    "General",
    "Interface",
    "File Handling",
    "Performance",
    "Cursors",
    "Transparency & Gamut",
    "Units & Rulers",
    "Guides, Grid, & Slices",
    "Plug-ins",
    "Type",
    "3D",
    "Agent Control",
    "Updates",
]

STATUS_LABELS = [
    "Document Sizes",
    "Document Dimensions",
    "Measurement Scale",
    "Scratch Sizes",
    "Efficiency",
]

ROW = re.compile(
    r'CommandSpec \{ id: "([^"]*)", path: &\[(.*?)\], label: "([^"]*)", '
    r'shortcut: "([^"]*)", implemented: (true|false), checkable: (true|false) \}'
)
MANIFEST = re.compile(r"<!--\s*pictura-commands\s*(.*?)-->", re.S)
PATH_PART = re.compile(r'"([^"]*)"')


def parse_catalogue(path: Path) -> dict[str, dict]:
    rows: dict[str, dict] = {}
    if not path.is_file():
        return rows
    for line in path.read_text(encoding="utf-8").splitlines():
        match = ROW.search(line)
        if not match:
            continue
        command_id, path_text, label, shortcut, implemented, _checkable = match.groups()
        rows[command_id] = {
            "id": command_id,
            "path": PATH_PART.findall(path_text),
            "label": label,
            "shortcut": shortcut,
            "implemented": implemented == "true",
        }
    return rows


def parse_manifests(docs: Path) -> dict[str, list[str]]:
    manifests: dict[str, list[str]] = {}
    if not docs.is_dir():
        return manifests
    for page in sorted(docs.glob("*.md")):
        text = page.read_text(encoding="utf-8")
        for block in MANIFEST.findall(text):
            for command_id in block.split():
                manifests.setdefault(command_id, []).append(page.name)
    return manifests


def parse_allowlist(path: Path) -> dict[str, str]:
    entries: dict[str, str] = {}
    if not path.is_file():
        return entries
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 1)
        entries[parts[0]] = parts[1].strip() if len(parts) > 1 else ""
    return entries


def page_for(command_id: str, path: list[str]) -> str | None:
    if command_id.startswith("edit.preferences."):
        return "preferences.md"
    if not path:
        return None
    return PAGE_FOR_TOP.get(path[0])


def check_public_docs(root: Path = ROOT) -> list[str]:
    catalogue = parse_catalogue(root / ENTRIES)
    manifests = parse_manifests(root / DOCS)
    allowlist = parse_allowlist(root / ALLOWLIST)
    errors: list[str] = []

    if not catalogue:
        return [f"no commands parsed from {ENTRIES}"]

    for command_id, entry in catalogue.items():
        path = entry["path"]
        page = page_for(command_id, path)
        pages = manifests.get(command_id, [])
        if page is None:
            if command_id not in allowlist:
                errors.append(
                    f"{command_id}: internal command has no allowlist entry in {ALLOWLIST}"
                )
            elif not allowlist[command_id]:
                errors.append(f"{command_id}: allowlist entry has no reason")
            continue
        if command_id in allowlist:
            errors.append(
                f"{command_id}: has a menu path ({' > '.join(path)}) and must be "
                f"documented on {page}, not allowlisted"
            )
        if not pages:
            errors.append(f"{command_id}: missing from {page}")
        elif page not in pages:
            errors.append(
                f"{command_id}: filed under {', '.join(pages)} but belongs on {page}"
            )
        else:
            elsewhere = sorted(set(pages) - {page})
            if elsewhere:
                errors.append(
                    f"{command_id}: also filed under {', '.join(elsewhere)}; "
                    f"a menu command belongs only on {page}"
                )

    for command_id in manifests:
        if command_id not in catalogue:
            errors.append(f"{command_id}: manifest id is not in the command catalogue")

    for command_id in allowlist:
        if command_id not in catalogue:
            errors.append(f"{command_id}: allowlist id is not in the command catalogue")
        elif catalogue[command_id]["path"]:
            errors.append(
                f"{command_id}: allowlisted command has a menu path; document it instead"
            )
        elif not allowlist[command_id]:
            errors.append(f"{command_id}: allowlist entry has no reason")

    view_page = root / DOCS / "menu-view.md"
    view_text = view_page.read_text(encoding="utf-8") if view_page.is_file() else ""
    for command_id, image in VIEW_EXTRAS.items():
        if command_id not in manifests:
            errors.append(f"{command_id}: missing from menu-view.md")
        if f"images/{image}" not in view_text:
            errors.append(f"{command_id}: menu-view.md does not reference images/{image}")
        if not (root / DOCS / "images" / image).is_file():
            errors.append(f"{command_id}: missing screenshot public_docs/images/{image}")

    preferences = root / DOCS / "preferences.md"
    preferences_text = preferences.read_text(encoding="utf-8") if preferences.is_file() else ""
    for label in PREFERENCES_LABELS:
        if label not in preferences_text:
            errors.append(f"preferences.md: missing Preferences page {label!r}")

    status = root / DOCS / "status-bar.md"
    status_text = status.read_text(encoding="utf-8") if status.is_file() else ""
    for label in STATUS_LABELS:
        if label not in status_text:
            errors.append(f"status-bar.md: missing readout {label!r}")

    for page in sorted((root / DOCS).glob("*.md")):
        text = page.read_text(encoding="utf-8")
        for target in re.findall(r"\]\(([^)#]+?)(?:#[^)]*)?\)", text):
            target = target.strip()
            if re.match(r"^[a-z][a-z0-9+.-]*:", target) or target.startswith("//"):
                continue
            if not (page.parent / target).resolve().is_file():
                errors.append(f"{page.name}: broken local link {target}")

    guards = root / GUARDS
    guards_text = guards.read_text(encoding="utf-8") if guards.is_file() else ""
    manifest_path = root / "scripts/checks.toml"
    if "gate_manifest.py run ci-guards" in guards_text and manifest_path.is_file():
        import tomllib
        rows = tomllib.loads(manifest_path.read_text(encoding="utf-8")).get("check", [])
        entry = next((row for row in rows if row.get("id") == "public-docs"), {})
        if (all(entry.get(key) for key in ("local_fast", "local_full", "docs_only", "ci_guards"))
                and "scripts/check-public-docs-commands.py --self-test" in entry.get("command", "")
                and "gate_manifest.py run" in (root / VERIFY_FAST).read_text(encoding="utf-8")):
            return errors
        errors.append("manifest: public docs check is missing required placements")
        return errors
    if "check-public-docs-commands.py" not in guards_text:
        errors.append(f"{GUARDS}: does not run scripts/check-public-docs-commands.py")
    if "check-public-docs-commands.py --self-test" not in guards_text:
        errors.append(f"{GUARDS}: does not run the guard's self-test")

    fast = root / VERIFY_FAST
    fast_text = fast.read_text(encoding="utf-8") if fast.is_file() else ""
    if "check-public-docs-commands.py" not in fast_text:
        errors.append(f"{VERIFY_FAST}: does not run scripts/check-public-docs-commands.py")

    return errors


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def self_test() -> int:
    entries = "\n".join(
        [
            '    CommandSpec { id: "file.new", path: &["File", "New…"], label: "New…", shortcut: "Ctrl+N", implemented: true, checkable: false },',
            '    CommandSpec { id: "file.close", path: &["File", "Close"], label: "Close", shortcut: "Ctrl+W", implemented: true, checkable: false },',
            '    CommandSpec { id: "edit.preferences.general", path: &["Edit", "Preferences", "General"], label: "General", shortcut: "Ctrl+K", implemented: true, checkable: false },',
            *[
                f'    CommandSpec {{ id: "{cid}", path: &["View", "Show", "{cid}"], label: "{cid}", shortcut: "", implemented: true, checkable: true }},'
                for cid in VIEW_EXTRAS
            ],
            '    CommandSpec { id: "panel.foo", path: &[], label: "Foo", shortcut: "", implemented: true, checkable: false },',
        ]
    )
    manifest = lambda *ids: "<!-- pictura-commands\n" + "\n".join(ids) + "\n-->\n"
    preferences = (
        "# Preferences\n\n"
        + "\n".join(f"- {label}" for label in PREFERENCES_LABELS)
        + "\n\n"
        + manifest("edit.preferences.general")
    )
    status = "# Status bar\n\n" + "\n".join(f"- {label}" for label in STATUS_LABELS) + "\n"
    view = (
        "# View\n\n"
        + "\n".join(f"![{cid}](images/{image})" for cid, image in VIEW_EXTRAS.items())
        + "\n\n"
        + manifest(*VIEW_EXTRAS)
    )

    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        write(root / ENTRIES, entries)
        write(root / DOCS / "menu-file.md", manifest("file.new", "file.close"))
        write(root / DOCS / "menu-edit.md", manifest())
        write(root / DOCS / "preferences.md", preferences)
        write(root / DOCS / "status-bar.md", status)
        write(root / DOCS / "menu-view.md", view)
        for image in VIEW_EXTRAS.values():
            write(root / DOCS / "images" / image, "")
        write(root / ALLOWLIST, "# internal ids\npanel.foo panel row action\n")
        write(root / GUARDS, "python3 scripts/check-public-docs-commands.py --self-test\n"
                             "python3 scripts/check-public-docs-commands.py\n")
        write(root / VERIFY_FAST, "python3 scripts/check-public-docs-commands.py\n")

        assert check_public_docs(root) == [], check_public_docs(root)

        write(root / DOCS / "menu-file.md", manifest("file.new"))
        errors = check_public_docs(root)
        assert any("file.close" in error and "menu-file.md" in error for error in errors), errors

        write(root / ALLOWLIST, "# internal ids\n")
        errors = check_public_docs(root)
        assert any("panel.foo" in error and "allowlist" in error for error in errors), errors

        write(root / ALLOWLIST, "# internal ids\npanel.foo\n")
        errors = check_public_docs(root)
        assert any("panel.foo" in error and "no reason" in error for error in errors), errors

        write(root / ALLOWLIST, "# internal ids\npanel.foo panel row action\nfile.close reason\n")
        errors = check_public_docs(root)
        assert any("file.close" in error and "must be documented" in error for error in errors), errors

        write(root / ALLOWLIST, "# internal ids\npanel.foo panel row action\n")
        write(root / DOCS / "menu-file.md", manifest("file.new", "file.close", "bogus.id"))
        errors = check_public_docs(root)
        assert any("bogus.id" in error and "not in the command catalogue" in error for error in errors), errors

        write(root / DOCS / "menu-file.md", manifest("file.new", "file.close"))
        write(root / DOCS / "menu-window.md", manifest("file.new"))
        errors = check_public_docs(root)
        assert any("file.new" in error and "menu-file.md" in error for error in errors), errors

        write(root / DOCS / "menu-window.md", manifest())
        (root / DOCS / "images" / VIEW_EXTRAS["view.show.layer.edges"]).unlink()
        errors = check_public_docs(root)
        assert any("view-show-layer-edges.png" in error for error in errors), errors

        write(root / DOCS / "images" / VIEW_EXTRAS["view.show.layer.edges"], "")
        write(root / DOCS / "status-bar.md", "# Status bar\n\n- Document Sizes\n")
        errors = check_public_docs(root)
        assert any("Efficiency" in error for error in errors), errors

        write(root / DOCS / "status-bar.md", status)
        write(root / DOCS / "preferences.md", "# Preferences\n\n- General\n")
        errors = check_public_docs(root)
        assert any("Updates" in error for error in errors), errors

        write(root / DOCS / "preferences.md", preferences)
        write(root / DOCS / "link-check.md", "# Link check\n\n[Missing](missing.md)\n")
        errors = check_public_docs(root)
        assert any("missing.md" in error for error in errors), errors
        (root / DOCS / "link-check.md").unlink()

    print("check-public-docs-commands: self-test OK")
    return 0


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test()
    errors = check_public_docs()
    for error in errors:
        print(f"FAIL: {error}")
    print(f"check-public-docs-commands: {len(errors)} error(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
