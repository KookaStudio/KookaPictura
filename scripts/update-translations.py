#!/usr/bin/env python3
"""Refresh the Linguist sources for the supported Pictura UI surfaces."""

import argparse
from collections import Counter
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[1]
CPP = ROOT / "crates/pictura-app/cpp"
SCRIPT = ROOT / "crates/pictura-script/src"
FILE_INFO = ROOT / "crates/pictura-app/src/cxxqt_object/file_info.rs"
CORE = ROOT / "crates/pictura-core/src/lib.rs"
TRANSLATIONS = ROOT / "translations"
STRING = r'"(?:[^"\\]|\\.)*"'
COMMAND_PATH = re.compile(r"path: &\[(.*?)\],\s*label:", re.DOTALL)
LABEL = re.compile(rf"\blabel: ({STRING})")
TOOL_TEXT = re.compile(rf"\b(?:label|hint): ({STRING})")
TOOL_MODIFIER_TEXT = re.compile(rf"\bh\(\s*{STRING},\s*({STRING})|\btext:\s*({STRING})")
METADATA_FIELD = re.compile(rf"\bpush\(\s*{STRING},\s*{STRING},\s*({STRING})")
METADATA_ROW = re.compile(r'format!\(\s*"[^"\\]+\\t([^"\\{]+)\\t')
BLEND_MODE = re.compile(rf"\bBlendMode::\w+\s*=>\s*({STRING})")
PLACEHOLDER = re.compile(r"%L?(?:[1-9][0-9]?|n)|\{[A-Za-z][A-Za-z0-9_]*\}|\*\.[A-Za-z0-9]+")


def catalogue_text():
    commands = (SCRIPT / "command_catalogue/entries.rs").read_text(encoding="utf-8")
    tools = (SCRIPT / "tool_catalogue/entries.rs").read_text(encoding="utf-8")
    modifiers = (SCRIPT / "tool_catalogue.rs").read_text(encoding="utf-8")
    metadata = FILE_INFO.read_text(encoding="utf-8")
    core = CORE.read_text(encoding="utf-8")
    command_text = set(LABEL.findall(commands))
    for path in COMMAND_PATH.findall(commands):
        command_text.update(re.findall(STRING, path))
    return {
        "PicturaCommands": {json.loads(text) for text in command_text},
        "PicturaTools": {json.loads(text) for text in TOOL_TEXT.findall(tools)}
        | {json.loads(first or second) for first, second in TOOL_MODIFIER_TEXT.findall(modifiers)},
        "PicturaBlendModes": {json.loads(text) for text in BLEND_MODE.findall(core)},
        "PicturaMetadata": {json.loads(text) for text in METADATA_FIELD.findall(metadata)}
        | set(METADATA_ROW.findall(metadata)),
    }


def check_catalog(path):
    failures = []
    for context in ElementTree.parse(path).findall("context"):
        for message in context.findall("message"):
            source = message.findtext("source", "")
            translation = message.find("translation")
            kind = translation.get("type")
            if kind in {"vanished", "obsolete"}:
                continue
            forms = translation.findall("numerusform") or [translation]
            expected = Counter(PLACEHOLDER.findall(source))
            if kind == "unfinished" or not all(form.text for form in forms):
                failures.append((context.findtext("name"), source, "unfinished"))
            elif any(Counter(PLACEHOLDER.findall(form.text)) != expected for form in forms):
                failures.append((context.findtext("name"), source, "placeholders or file patterns differ"))
    for context, source, reason in failures:
        print(f"{path.name}: {context}: {source!r}: {reason}")
    return bool(failures)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lupdate", default="lupdate", help="Qt 6 lupdate executable")
    parser.add_argument("--check", action="store_true",
                        help="check current coverage and placeholders without editing catalogs")
    parser.add_argument("catalogs", nargs="*", type=Path, help="TS files to update")
    args = parser.parse_args()
    catalogs = args.catalogs or [TRANSLATIONS / f"pictura_{lang}.ts" for lang in ("de", "fi")]
    sources = sorted(
        path for path in CPP.rglob("*")
        if path.suffix in {".cpp", ".h"}
        and "tests" not in path.relative_to(CPP).parts
        and not path.stem.startswith("selftest")
        and not path.stem.endswith("_test")
    )
    with tempfile.TemporaryDirectory(prefix="pictura-i18n-") as directory:
        temporary = Path(directory)
        anchors = temporary / "catalogue.cpp"
        entries = (
            f"{{{json.dumps(context)}, QT_TRANSLATE_NOOP({json.dumps(context)}, "
            f"{json.dumps(source, ensure_ascii=False)})}},\n"
            for context, messages in catalogue_text().items() for source in sorted(messages)
        )
        anchors.write_text(
            "const char* labels[][2] = {\n" + "".join(entries) + "};\n",
            encoding="utf-8",
        )
        targets = catalogs
        if args.check:
            targets = []
            for index, catalog in enumerate(catalogs):
                copy = temporary / str(index) / catalog.name
                copy.parent.mkdir()
                shutil.copyfile(catalog, copy)
                targets.append(copy)
        result = subprocess.call([
            args.lupdate, "-locations", "none", *(str(src) for src in sources),
            str(anchors), "-ts", *(str(path) for path in targets),
        ])
        if result or not args.check:
            return result
        failures = [check_catalog(path) for path in targets]
        return int(any(failures))


if __name__ == "__main__":
    raise SystemExit(main())
