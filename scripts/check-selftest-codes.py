#!/usr/bin/env python3
"""Pin the declarative self-test registries and failure-site ownership."""
from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = ROOT / "crates" / "pictura-app" / "cpp"
CODE_HEADER = SOURCES / "selftest_codes.h"
CHECK_HEADER = SOURCES / "selftest_checks.h"

BEGIN = re.compile(r"\b(?:ST_BEGIN|selfTest\s*\(\s*\)\s*\.\s*begin)\s*\(")
FAIL = re.compile(r"\bST_FAIL\s*\(")
DOT_FAIL = re.compile(r"\bselfTest\s*\(\s*\)\s*\.\s*fail\s*\(")
FAIL_RUN = re.compile(r"\bselfTest\s*\(\s*\)\s*\.\s*failRun\s*\(")
SKIP = re.compile(r"\bST_SKIP\s*\(")
CODE_ENTRY = re.compile(
    r'PICTURA_SELFTEST_CODE\(\s*(kCode\w+)\s*,\s*(\d+)\s*,\s*"([^"]*)"\s*\)'
)
CHECK_ENTRY = re.compile(r'PICTURA_SELFTEST_CHECK\(\s*"([^"]+)"\s*,\s*([^)]+?)\s*\)')
CONTROL_KEYWORDS = {"if", "for", "while", "switch", "catch"}


@dataclass
class Registry:
    constants: dict[str, int] = field(default_factory=dict)
    code_names: dict[int, set[str]] = field(default_factory=dict)
    checks: set[str] = field(default_factory=set)
    retired: set[int] = field(default_factory=set)


def _skip_string(text: str, i: int) -> int:
    quote = text[i]
    i += 1
    while i < len(text):
        if text[i] == "\\":
            i += 2
            continue
        if text[i] == quote:
            return i + 1
        i += 1
    return len(text)


def strip_comments(text: str) -> str:
    out = []
    i = 0
    while i < len(text):
        if text[i] in ('"', "'"):
            end = _skip_string(text, i)
            out.append(text[i:end])
            i = end
            continue
        if text.startswith("//", i):
            while i < len(text) and text[i] != "\n":
                out.append(" ")
                i += 1
            continue
        if text.startswith("/*", i):
            out.append("  ")
            i += 2
            while i < len(text) and not text.startswith("*/", i):
                out.append("\n" if text[i] == "\n" else " ")
                i += 1
            if i < len(text):
                out.append("  ")
                i += 2
            continue
        out.append(text[i])
        i += 1
    return "".join(out)


def call_args(text: str, open_paren: int) -> str | None:
    depth = 0
    i = open_paren
    while i < len(text):
        if text[i] in ('"', "'"):
            i = _skip_string(text, i)
            continue
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
            if depth == 0:
                return text[open_paren + 1:i]
        i += 1
    return None


def _split_args(raw: str) -> list[str]:
    args = []
    start = 0
    depth = 0
    i = 0
    while i < len(raw):
        if raw[i] in ('"', "'"):
            i = _skip_string(raw, i)
            continue
        if raw[i] == "(":
            depth += 1
        elif raw[i] == ")":
            depth -= 1
        elif raw[i] == "," and depth == 0:
            args.append(raw[start:i].strip())
            start = i + 1
        i += 1
    args.append(raw[start:].strip())
    return args


def _literal_name(raw: str) -> str | None:
    match = re.fullmatch(r'\s*"((?:\\.|[^"\\])*)"\s*', raw)
    return match.group(1) if match else None


def _constant_name(raw: str) -> str | None:
    match = re.fullmatch(r"\s*(?:pictura::selftest_codes::)?(kCode\w+)\s*", raw)
    return match.group(1) if match else None


def parse_registry(code_text: str, check_text: str) -> tuple[Registry, list[str]]:
    registry = Registry()
    errors = []
    for match in CODE_ENTRY.finditer(code_text):
        name, value_text, checks_text = match.groups()
        value = int(value_text)
        if name in registry.constants:
            errors.append(f"selftest_codes.h: duplicate constant {name}")
        if value in registry.code_names:
            errors.append(f"selftest_codes.h: duplicate code {value}")
        registry.constants[name] = value
        registry.code_names[value] = set(checks_text.split("|"))
    retired_match = re.search(r"kRetiredCodes\[\]\s*=\s*\{([^}]*)\}", code_text)
    if retired_match:
        registry.retired = {int(value) for value in re.findall(r"\d+", retired_match.group(1))}
    for match in CHECK_ENTRY.finditer(check_text):
        name, suite = match.groups()
        if name in registry.checks:
            errors.append(f"selftest_checks.h: duplicate check {name}")
        if not suite.strip():
            errors.append(f"selftest_checks.h: check {name} has no suite")
        registry.checks.add(name)
    for value, names in registry.code_names.items():
        if value <= 0:
            errors.append(f"selftest_codes.h: code {value} is not positive")
        if value in registry.retired:
            errors.append(f"selftest_codes.h: active code {value} is retired")
        for name in names:
            if name not in registry.checks:
                errors.append(f"selftest_codes.h: code {value} names unknown check {name}")
    return registry, errors


def load_registry() -> tuple[Registry, list[str]]:
    return parse_registry(CODE_HEADER.read_text(encoding="utf-8"), CHECK_HEADER.read_text(encoding="utf-8"))


def _string_arrays(clean: str) -> dict[str, list[str]]:
    arrays = {}
    pattern = re.compile(r"\bconst\s+char\s*\*\s*(\w+)\s*\[[^\]]*\]\s*=\s*\{([^}]*)\}")
    for match in pattern.finditer(clean):
        arrays[match.group(1)] = re.findall(r'"((?:\\.|[^"\\])*)"', match.group(2))
    return arrays


def _code_arrays(clean: str) -> dict[str, list[str]]:
    arrays = {}
    pattern = re.compile(r"\bconst\s+int\s+(\w+)\s*\[[^\]]*\]\s*=\s*\{([^}]*)\}")
    for match in pattern.finditer(clean):
        arrays[match.group(1)] = [item.strip() for item in match.group(2).split(",") if item.strip()]
    return arrays


def _begin_sites(clean: str) -> list[dict]:
    names = _string_arrays(clean)
    sites = []
    for match in BEGIN.finditer(clean):
        raw = call_args(clean, match.end() - 1) or ""
        args = _split_args(raw)
        name = _literal_name(args[0]) if args else None
        indexed = re.fullmatch(r"\s*(\w+)\s*\[\s*(\w+)\s*\]\s*", raw)
        sites.append({
            "offset": match.start(),
            "name": name,
            "array": names.get(indexed.group(1)) if indexed else None,
            "index": indexed.group(2) if indexed else None,
        })
    return sites


def _resolve_code(raw: str, registry: Registry, arrays: dict[str, list[str]]) -> tuple[list[int], str | None]:
    constant = _constant_name(raw)
    if constant:
        if constant not in registry.constants:
            return [], constant
        return [registry.constants[constant]], None
    indexed = re.fullmatch(r"\s*(\w+)\s*\[\s*(\w+)\s*\]\s*", raw)
    if indexed and indexed.group(1) in arrays:
        values = arrays[indexed.group(1)]
        index = indexed.group(2)
        if index.isdigit():
            position = int(index)
            if position >= len(values):
                return [], None
            constant = _constant_name(values[position])
            if constant and constant in registry.constants:
                return [registry.constants[constant]], None
            return [], constant
        resolved = []
        for value in values:
            constant = _constant_name(value)
            if constant and constant in registry.constants:
                resolved.append(registry.constants[constant])
            else:
                return [], constant
        return resolved, None
    return [], None


def _brace_blocks(text: str) -> list[tuple[int, int]]:
    stack = []
    blocks = []
    i = 0
    while i < len(text):
        if text[i] in ('"', "'"):
            i = _skip_string(text, i)
            continue
        if text[i] == "{":
            stack.append(i)
        elif text[i] == "}" and stack:
            blocks.append((stack.pop(), i))
        i += 1
    return blocks


def _matching_brace(text: str, open_offset: int) -> int | None:
    depth = 0
    i = open_offset
    while i < len(text):
        if text[i] in ('"', "'"):
            i = _skip_string(text, i)
            continue
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return None


def _looks_like_function_body(text: str, brace: int) -> bool:
    j = brace - 1
    while j >= 0 and text[j] in " \t\r\n":
        j -= 1
    while True:
        tail = re.search(r"\b(const|noexcept|override|final)\b\s*$", text[:j + 1])
        if not tail:
            break
        j = tail.start() - 1
        while j >= 0 and text[j] in " \t\r\n":
            j -= 1
    if j < 0 or text[j] != ")":
        return False
    depth = 0
    k = j
    while k >= 0:
        if text[k] == ")":
            depth += 1
        elif text[k] == "(":
            depth -= 1
            if depth == 0:
                break
        k -= 1
    if k < 0:
        return True
    word = re.search(r"(\w+)\s*$", text[:k])
    return not (word and word.group(1) in CONTROL_KEYWORDS)


def _enclosing_function_body(text: str, offset: int) -> tuple[int, int] | None:
    stack = []
    i = 0
    while i < len(text) and i < offset:
        if text[i] in ('"', "'"):
            i = _skip_string(text, i)
            continue
        if text[i] == "{":
            stack.append((i, _looks_like_function_body(text, i)))
        elif text[i] == "}" and stack:
            stack.pop()
        i += 1
    for open_offset, is_function in reversed(stack):
        if is_function:
            close = _matching_brace(text, open_offset)
            if close is not None:
                return open_offset, close
    return None


def _check_comment_offset(text: str, name: str, before: int, after: int) -> int | None:
    pattern = re.compile(rf"^[ \t]*//[ \t]*{re.escape(name)}[ \t]*\(", re.MULTILINE)
    found = None
    for match in pattern.finditer(text, after, before):
        found = match.start()
    return found


def check_source(text: str, label: str, registry: Registry) -> tuple[list[str], set[int], set[str]]:
    clean = strip_comments(text)
    line_of = lambda offset: clean.count("\n", 0, offset) + 1
    arrays = _code_arrays(clean)
    begins = _begin_sites(clean)
    errors = []
    used_codes: set[int] = set()
    used_checks: set[str] = set()
    for site in begins:
        if site["name"]:
            used_checks.add(site["name"])
            if site["name"] not in registry.checks:
                errors.append(f"{label}:{line_of(site['offset'])}: unregistered check {site['name']}")
        elif site["array"]:
            used_checks.update(site["array"])
            for name in site["array"]:
                if name not in registry.checks:
                    errors.append(f"{label}:{line_of(site['offset'])}: unregistered check {name}")
        else:
            errors.append(f"{label}:{line_of(site['offset'])}: ST_BEGIN has no registry-owned name")

    failures = []
    for pattern in (FAIL, DOT_FAIL, FAIL_RUN):
        for match in pattern.finditer(clean):
            raw = call_args(clean, match.end() - 1)
            if raw is None:
                errors.append(f"{label}:{line_of(match.start())}: unterminated failure call")
                continue
            failures.append((match.start(), pattern is FAIL_RUN, _split_args(raw)))
    failures.sort()

    reported = set()
    for offset, is_run, args in failures:
        if is_run:
            if len(args) < 2:
                errors.append(f"{label}:{line_of(offset)}: failRun needs a name and code")
                continue
            owner = _literal_name(args[0])
            code_raw = args[1]
            owners = [owner] if owner else []
            paired_codes = None
        else:
            if not args:
                errors.append(f"{label}:{line_of(offset)}: failure call has no code")
                continue
            prior = [site for site in begins if site["offset"] < offset]
            if not prior:
                errors.append(f"{label}:{line_of(offset)}: failure site before any ST_BEGIN")
                continue
            site = prior[-1]
            owner = site["name"]
            code_raw = args[0]
            owners = [owner] if owner else []
            paired_codes = None
            if owner is None and site["array"]:
                index_match = re.fullmatch(r"\s*(\w+)\s*\[\s*(\w+)\s*\]\s*", code_raw)
                if index_match:
                    position = index_match.group(2)
                    if position.isdigit():
                        index = int(position)
                        if index < len(site["array"]):
                            owners = [site["array"][index]]
                    elif position == site["index"]:
                        resolved, unresolved = _resolve_code(code_raw, registry, arrays)
                        if not unresolved and len(resolved) == len(site["array"]):
                            owners = site["array"]
                            paired_codes = resolved
        if not owners:
            errors.append(f"{label}:{line_of(offset)}: failure site has no resolvable check owner")
            continue
        used_checks.update(owners)
        for owner in owners:
            if owner not in registry.checks:
                errors.append(f"{label}:{line_of(offset)}: unregistered check {owner}")
        codes, unresolved = (
            (paired_codes, None) if paired_codes is not None else _resolve_code(code_raw, registry, arrays)
        )
        if re.fullmatch(r"\s*(?:0[xX][0-9a-fA-F]+|\d+)\s*", code_raw):
            errors.append(f"{label}:{line_of(offset)}: failure code must be a named registry constant")
        elif unresolved:
            errors.append(f"{label}:{line_of(offset)}: unregistered failure constant {unresolved}")
        elif not codes:
            errors.append(f"{label}:{line_of(offset)}: unresolved failure code {code_raw}")
        for index, code in enumerate(codes):
            used_codes.add(code)
            owner = owners[index] if paired_codes is not None else owners[0]
            if code in registry.retired:
                errors.append(f"{label}:{line_of(offset)}: retired failure code {code}")
            if code not in registry.code_names:
                errors.append(f"{label}:{line_of(offset)}: unregistered failure code {code}")
            elif owner not in registry.code_names[code]:
                errors.append(
                    f"{label}:{line_of(offset)}: code {code} is not allocated to check {owner}"
                )

    blocks = _brace_blocks(clean)
    begin_offsets = [site["offset"] for site in begins]
    for offset, is_run, _args in failures:
        if is_run or not begins or offset < begins[0]["offset"]:
            continue
        for open_offset, close_offset in sorted(
            (block for block in blocks if block[0] < offset < block[1]), key=lambda block: -block[0]
        ):
            inside = [begin for begin in begin_offsets if open_offset < begin < close_offset]
            if not inside:
                continue
            first = min(inside)
            if offset < first:
                name = next(site["name"] for site in begins if site["offset"] == first)
                errors.append(
                    f"{label}:{line_of(offset)}: failure site precedes its own check's ST_BEGIN ({name})"
                )
                reported.add(offset)
            break

    for index, site in enumerate(begins):
        name = site["name"]
        if not name:
            continue
        floor = begins[index - 1]["offset"] if index else 0
        introduced = _check_comment_offset(text, name, site["offset"], floor)
        if introduced is None:
            continue
        for offset, is_run, _args in failures:
            if not is_run and introduced < offset < site["offset"] and offset not in reported:
                errors.append(
                    f"{label}:{line_of(offset)}: failure site precedes its own check's ST_BEGIN ({name})"
                )
                reported.add(offset)

    for skip in SKIP.finditer(clean):
        args = call_args(clean, skip.end() - 1)
        if args is None:
            continue
        tail = clean[skip.end() + len(args) + 1:]
        next_statement = re.match(r"[\s;}]*?(return\b[^;]*;)", tail, re.S)
        if next_statement is None:
            continue
        after_return = skip.end() + len(args) + 1 + next_statement.end()
        body = _enclosing_function_body(clean, skip.start())
        limit = body[1] if body else len(clean)
        if BEGIN.search(clean, after_return, limit):
            errors.append(
                f"{label}:{line_of(skip.start())}: ST_SKIP followed by a return before later checks"
            )
    return errors, used_codes, used_checks


def check_sources(directory: Path = SOURCES) -> list[str]:
    registry, errors = load_registry()
    used_codes: set[int] = set()
    used_checks: set[str] = set()
    paths = sorted(directory.glob("selftest*.cpp")) + [directory / "main.cpp"]
    for path in paths:
        if path.name in {"selftest_report.cpp", "selftest_run.cpp"}:
            continue
        source_errors, codes, checks = check_source(path.read_text(encoding="utf-8"), path.name, registry)
        errors.extend(source_errors)
        used_codes.update(codes)
        used_checks.update(checks)
    for code in sorted(set(registry.code_names) - used_codes):
        errors.append(f"selftest_codes.h: allocated code {code} has no failure site")
    for name in sorted(registry.checks - used_checks):
        errors.append(f"selftest_checks.h: registered check {name} has no ST_BEGIN or failRun")
    return errors


FIXTURE_CODES = """
PICTURA_SELFTEST_CODE(kCode301, 301, "first|second")
PICTURA_SELFTEST_CODE(kCode302, 302, "second")
PICTURA_SELFTEST_CODE(kCode303, 303, "startup")
inline constexpr int kRetiredCodes[] = {25};
"""
FIXTURE_CHECKS = """
PICTURA_SELFTEST_CHECK("first", kLegacySuite)
PICTURA_SELFTEST_CHECK("second", kLegacySuite)
PICTURA_SELFTEST_CHECK("startup", kLegacySuite)
"""


def self_check() -> int:
    registry, registry_errors = parse_registry(FIXTURE_CODES, FIXTURE_CHECKS)
    cases = [
        ("clean", """
int run() {
    ST_BEGIN("first");
    if (!ready()) { return selfTest().fail(pictura::selftest_codes::kCode301, "fixture"); }
    ST_PASS("ok");
    ST_BEGIN("second");
    if (!ready()) { return selfTest().fail(pictura::selftest_codes::kCode302, "fixture"); }
    ST_PASS("ok");
    return 0;
}
""", 0),
        ("numeric code", """
int run() { ST_BEGIN("first"); return selfTest().fail(301, "bad"); }
""", 1),
        ("zero code", """
int run() { ST_BEGIN("first"); return selfTest().fail(0, "bad"); }
""", 1),
        ("negative code", """
int run() { ST_BEGIN("first"); return selfTest().fail(-1, "bad"); }
""", 1),
        ("unregistered constant", """
int run() { ST_BEGIN("first"); return selfTest().fail(pictura::selftest_codes::kCode999, "bad"); }
""", 1),
        ("code check mismatch", """
int run() { ST_BEGIN("first"); return selfTest().fail(pictura::selftest_codes::kCode302, "bad"); }
""", 1),
        ("shared owner", """
int run() { ST_BEGIN("second"); return selfTest().fail(pictura::selftest_codes::kCode301, "shared"); }
""", 0),
        ("unbraced next check fixture", """
int run() {
    ST_BEGIN("first");
    ST_PASS("ok");
    if (!ready()) { return selfTest().fail(pictura::selftest_codes::kCode302, "second fixture"); }
    ST_BEGIN("second");
    ST_PASS("ok");
    return 0;
}
""", 1),
        ("indexed table", """
int run() {
    const int codes[2] = {pictura::selftest_codes::kCode301, pictura::selftest_codes::kCode302};
    const char* names[2] = {"first", "second"};
    for (int i = 0; i < 2; ++i) {
        ST_BEGIN(names[i]);
        if (!ready()) { return selfTest().fail(codes[i], "fixture"); }
        ST_PASS("ok");
    }
    return 0;
}
""", 0),
        ("startup helper", """
int main() { return selfTest().failRun("startup", pictura::selftest_codes::kCode303, "bad"); }
""", 0),
        ("skip truncates", """
int run() {
    ST_BEGIN("first");
    ST_SKIP("no adapter");
    return 0;
    ST_BEGIN("second");
    ST_PASS("ok");
    return 0;
}
""", 1),
    ]
    failures = 0
    if registry_errors:
        failures += 1
        print(f"check-selftest-codes: fixture registry invalid: {registry_errors}", file=sys.stderr)
    for name, source, expected in cases:
        errors, _codes, _checks = check_source(source, name, registry)
        got = 1 if errors else 0
        if got != expected:
            failures += 1
            print(
                f"check-selftest-codes: self-test FAILED for {name!r}: "
                f"expected {'a violation' if expected else 'clean'}, got {errors}",
                file=sys.stderr,
            )
    duplicate_codes = FIXTURE_CODES + '\nPICTURA_SELFTEST_CODE(kCodeOther, 301, "first")\n'
    _registry, duplicate_errors = parse_registry(duplicate_codes, FIXTURE_CHECKS)
    if not any("duplicate code 301" in error for error in duplicate_errors):
        failures += 1
        print("check-selftest-codes: duplicate code fixture was accepted", file=sys.stderr)
    retired_codes = FIXTURE_CODES.replace("kCode301, 301", "kCode301, 25")
    _registry, retired_errors = parse_registry(retired_codes, FIXTURE_CHECKS)
    if not any("active code 25 is retired" in error for error in retired_errors):
        failures += 1
        print("check-selftest-codes: retired code fixture was accepted", file=sys.stderr)
    if failures:
        return 1
    print(f"check-selftest-codes: self-test OK ({len(cases) + 2} fixtures)")
    return 0


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Pin the self-test failure registries")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return self_check()
    errors = check_sources()
    for error in errors:
        print(f"check-selftest-codes: FAILED: {error}", file=sys.stderr)
    if errors:
        return 1
    print("check-selftest-codes: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
