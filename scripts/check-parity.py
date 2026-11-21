#!/usr/bin/env python3
"""Validate PARITY.md against its own rows, sources, examples, and gates.

Run from the repository root: python3 scripts/check-parity.py [--self-check]
"""
from __future__ import annotations

import copy
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PARITY = ROOT / "PARITY.md"
README = ROOT / "README.md"
ROADMAP = ROOT / "ROADMAP.md"
GAPS = ROOT / "docs/dev/doc-code-parity-gaps.md"
VERIFY_FAST = ROOT / "scripts/verify-fast.sh"
GUARDS = ROOT / ".github/workflows/guards.yml"

STATUSES = {"Complete", "Partial", "Missing", "Unknown"}
SCOPES = {"in-scope", "non-goal"}
BASELINES = (("CS6 Standard", "s"), ("CS6 Extended", "e"), ("CC 28.0", "cc"))
EXAMPLE_KINDS = (
    "complete workflow",
    "partial implementation",
    "unknown assessment",
    "intentional non-goal",
    "CC-only capability",
    "retired capability",
)


@dataclass
class Row:
    id: str
    feature: str
    s: str
    e: str
    cc: str
    status: str
    scope: str
    evidence: str
    check: str
    notes: str
    tracking: str
    category: str


def strip_cells(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def parse_sections(text: str) -> list[dict]:
    sections: list[dict] = []
    current = None
    for line in text.splitlines():
        match = re.match(r"^(#{2,3}) (.+)$", line)
        if match:
            current = {"level": len(match.group(1)), "title": match.group(2).strip(), "lines": []}
            sections.append(current)
        elif current is not None:
            current["lines"].append(line)
    return sections


def section_body(sections: list[dict], title: str) -> str:
    for section in sections:
        if section["title"] == title:
            return "\n".join(section["lines"])
    return ""


def parse_table(body: str) -> list[list[str]]:
    lines = [line for line in body.splitlines() if line.strip().startswith("|")]
    if len(lines) < 2:
        return []
    rows = []
    for line in lines[1:]:
        cells = strip_cells(line)
        if cells and all(set(cell) <= set("-: ") for cell in cells if cell):
            continue
        rows.append(cells)
    return rows


def header_value(text: str, key: str) -> str:
    match = re.search(rf"^{re.escape(key)}:\s*(.+)$", text, re.MULTILINE)
    return match.group(1).strip() if match else ""


def parse_feature_rows(text: str) -> tuple[list[Row], list[str]]:
    rows: list[Row] = []
    errors: list[str] = []
    for section in parse_sections(text):
        if section["level"] != 3:
            continue
        for cells in parse_table("\n".join(section["lines"])):
            if not cells or not cells[0].startswith("P-"):
                continue
            if len(cells) != 11:
                errors.append(f"{section['title']}: row {cells[0]} has {len(cells)} cells, expected 11")
                continue
            rows.append(Row(*cells, category=section["title"]))
    return rows, errors


def parse_headline(text: str) -> dict[str, dict]:
    sections = parse_sections(text)
    body = section_body(sections, "Headline")
    headline: dict[str, dict] = {}
    for cells in parse_table(body):
        if len(cells) != 8:
            continue
        try:
            headline[cells[0]] = {
                "complete": int(cells[1]),
                "partial": int(cells[2]),
                "missing": int(cells[3]),
                "unknown": int(cells[4]),
                "denominator": int(cells[5]),
                "coverage": cells[6],
                "excluded": int(cells[7]),
            }
        except ValueError:
            continue
    return headline


def parse_categories(text: str) -> dict[str, dict[str, str]]:
    sections = parse_sections(text)
    body = section_body(sections, "Category summary")
    categories: dict[str, dict[str, str]] = {}
    for cells in parse_table(body):
        if len(cells) != 4:
            continue
        categories[cells[0]] = {
            "CS6 Standard": cells[1],
            "CS6 Extended": cells[2],
            "CC 28.0": cells[3],
        }
    return categories


def parse_examples(text: str) -> list[dict[str, str]]:
    sections = parse_sections(text)
    body = section_body(sections, "Worked examples")
    examples = []
    for cells in parse_table(body):
        if len(cells) != 3:
            continue
        examples.append({"kind": cells[0].lower(), "row": cells[1].strip("`"), "source": cells[2]})
    return examples


def parse_history(text: str) -> list[dict[str, str]]:
    sections = parse_sections(text)
    body = section_body(sections, "Baseline history")
    history = []
    for cells in parse_table(body):
        if len(cells) != 6:
            continue
        history.append(
            {
                "date": cells[0],
                "commit": cells[1],
                "cs6s": cells[2],
                "cs6e": cells[3],
                "cc_version": cells[4],
                "cc": cells[5],
            }
        )
    return history


def parse_sources(text: str) -> int:
    sections = parse_sections(text)
    body = section_body(sections, "Sources")
    return len(re.findall(r"^\d+\.\s+", body, re.MULTILINE))


def baseline_header(header: dict[str, str], baseline_body: str) -> list[str]:
    errors = []
    required = ("Assessed", "Pictura commit", "CS6 baseline", "CC baseline")
    for key in required:
        if not header.get(key):
            errors.append(f"baseline header: missing {key}")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", header.get("Assessed", "")):
        errors.append("baseline header: Assessed is not YYYY-MM-DD")
    if not re.fullmatch(r"[0-9a-f]{7,40}", header.get("Pictura commit", "")):
        errors.append("baseline header: Pictura commit is not a hex commit")
    if "13.0" not in header.get("CS6 baseline", ""):
        errors.append("baseline header: CS6 baseline is not CS6 13.0")
    if "28.0" not in header.get("CC baseline", "") or "2026" not in header.get("CC baseline", ""):
        errors.append("baseline header: CC baseline is not Photoshop 28.0 (2026)")
    if not re.search(r"src:\d+", baseline_body):
        errors.append("baseline header: the baseline section does not cite a source")
    return errors


LINK = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")


def links_present(readme: str, roadmap: str) -> list[str]:
    errors = []
    wanted = (ROOT / "PARITY.md").resolve()
    for name, text in (("README.md", readme), ("ROADMAP.md", roadmap)):
        linked = False
        for label, target in LINK.findall(text):
            if label.strip().strip("`").strip().lower() != "parity.md":
                continue
            target = target.strip()
            if target.startswith(("http://", "https://", "#")):
                continue
            if (ROOT / target.split("#", 1)[0]).resolve() == wanted:
                linked = True
                break
        if not linked:
            errors.append(f"{name} does not link PARITY.md")
    return errors


def parity_disclaimer(text: str) -> list[str]:
    if not re.search(r"not behaviou?ral or pixel parity", text):
        return ["PARITY.md does not state that coverage is not behavioral or pixel parity"]
    return []


def duplicate_ids_rejected(rows: list[Row]) -> list[str]:
    seen: set[str] = set()
    errors = []
    for row in rows:
        if row.id in seen:
            errors.append(f"{row.id}: duplicate ID")
        seen.add(row.id)
    return errors


def na_requires_reason(row: Row) -> list[str]:
    errors = []
    notes = row.notes
    if row.s == "N/A" and row.e == "Y" and "edition:" not in notes:
        errors.append(f"{row.id}: Standard N/A lacks edition: reason")
    if row.s == "N/A" and row.e == "N/A" and "post-CS6:" not in notes:
        errors.append(f"{row.id}: CS6 N/A lacks post-CS6: reason")
    if row.cc == "N/A" and "retired:" not in notes:
        errors.append(f"{row.id}: CC N/A lacks retired: reason")
    if (row.s == "N/A" and row.e == "N/A") or row.cc == "N/A":
        if not re.search(r"src:\d+", notes):
            errors.append(f"{row.id}: N/A baseline lacks a source")
    return errors


def evidence_paths_exist(row: Row, root: Path = ROOT) -> list[str]:
    errors = []
    evidence = row.evidence.strip()
    if row.status == "Complete" and (not evidence or evidence == "—"):
        errors.append(f"{row.id}: Complete row has no evidence")
        return errors
    if not evidence or evidence == "—":
        return errors
    for path in evidence.split(";"):
        path = path.strip().strip("`")
        if not path or path == "—":
            continue
        if not (root / path).exists():
            errors.append(f"{row.id}: evidence path does not exist: {path}")
    return errors


def _test_reference_exists(owner: str, name: str) -> bool:
    if owner.startswith("tst_"):
        source = ROOT / "crates/pictura-app/cpp/tests" / f"{owner}.cpp"
        return source.is_file() and re.search(rf"\bvoid\s+{name}\s*\(", source.read_text(encoding="utf-8")) is not None
    script = ROOT / "scripts" / f"{owner}.py"
    if script.is_file():
        return re.search(rf"^def\s+{name}\s*\(", script.read_text(encoding="utf-8"), re.MULTILINE) is not None
    crate = ROOT / "crates" / owner
    if not crate.is_dir():
        return False
    return any(re.search(rf"\bfn\s+{name}\b", path.read_text(encoding="utf-8", errors="ignore")) for path in crate.rglob("*.rs"))


def check_field_valid(row: Row) -> list[str]:
    value = row.check.strip()
    if value == "none":
        return []
    if value.startswith("manual:"):
        return [] if value[len("manual:"):].strip() else [f"{row.id}: manual check has no reason"]
    if value.startswith("automated:"):
        reference = value[len("automated:"):].strip()
        if "::" not in reference:
            return [f"{row.id}: automated check has no owner::name reference"]
        owner, name = reference.split("::", 1)
        if not _test_reference_exists(owner, name):
            return [f"{row.id}: automated check does not resolve: {reference}"]
        return []
    return [f"{row.id}: invalid check value: {value}"]


def non_complete_tracks(row: Row, gaps_text: str) -> list[str]:
    if row.scope == "non-goal":
        if not row.tracking.startswith("non-goal:"):
            return [f"{row.id}: non-goal row lacks a non-goal: tracking citation"]
        if "ROADMAP.md:" not in row.tracking and "feasibility:" not in row.tracking:
            return [f"{row.id}: non-goal tracking does not cite ROADMAP.md or feasibility"]
        return []
    if row.status == "Complete":
        return []
    if re.fullmatch(r"#\d+", row.tracking):
        return []
    if row.tracking.startswith("follow-up:") and row.tracking[len("follow-up:"):].strip():
        return []
    gap_ids = set(re.findall(r"\b[A-Z]{2,5}-\d+\b", gaps_text))
    if row.tracking in gap_ids:
        return []
    return [f"{row.id}: in-scope {row.status} row lacks a gap ID, issue, or follow-up"]


def conflict_applicability(row: Row) -> list[str]:
    if "conflict:" not in row.notes:
        return []
    errors = []
    if row.status != "Unknown":
        errors.append(f"{row.id}: conflict row must be Unknown")
    if row.s != "Y" or row.e != "Y" or row.cc != "Y":
        errors.append(f"{row.id}: conflict row must be Y on every baseline")
    if not re.search(r"conflict:\d+", row.notes):
        errors.append(f"{row.id}: conflict note lacks a source number")
    return errors


def token_errors(row: Row, sources: int) -> list[str]:
    errors = []
    for match in re.finditer(r"src:(\d+)", row.notes):
        if not 1 <= int(match.group(1)) <= sources:
            errors.append(f"{row.id}: src:{match.group(1)} is outside the Sources list")
    for match in re.finditer(r"conflict:(\d+)", row.notes):
        if not 1 <= int(match.group(1)) <= sources:
            errors.append(f"{row.id}: conflict:{match.group(1)} is outside the Sources list")
    for match in re.finditer(r"members:(\d+)/(\d+)", row.notes):
        if int(match.group(1)) > int(match.group(2)):
            errors.append(f"{row.id}: members:{match.group(1)}/{match.group(2)} is impossible")
    return errors


def pct(complete: int, denominator: int) -> str:
    if denominator == 0:
        return "—"
    value = (2000 * complete + denominator) // (2 * denominator)
    return f"{value / 10:.1f}%"


def _counts(rows: list[Row]) -> dict[str, int]:
    return {
        "complete": sum(1 for row in rows if row.status == "Complete"),
        "partial": sum(1 for row in rows if row.status == "Partial"),
        "missing": sum(1 for row in rows if row.status == "Missing"),
        "unknown": sum(1 for row in rows if row.status == "Unknown"),
        "denominator": len(rows),
    }


def expected_headline(rows: list[Row]) -> dict[str, dict]:
    expected: dict[str, dict] = {}
    for label, key in BASELINES:
        applicable = [row for row in rows if getattr(row, key) == "Y"]
        counts = _counts(applicable)
        expected[label] = {
            **counts,
            "coverage": pct(counts["complete"], counts["denominator"]),
            "excluded": sum(1 for row in applicable if row.scope == "non-goal"),
        }
        in_scope = [row for row in applicable if row.scope == "in-scope"]
        in_counts = _counts(in_scope)
        expected[f"{label}, in scope"] = {
            **in_counts,
            "coverage": pct(in_counts["complete"], in_counts["denominator"]),
            "excluded": sum(1 for row in applicable if row.scope == "non-goal"),
        }
    return expected


def expected_categories(rows: list[Row]) -> dict[str, dict[str, str]]:
    expected: dict[str, dict[str, str]] = {}
    for category in dict.fromkeys(row.category for row in rows):
        expected[category] = {}
        for label, key in BASELINES:
            applicable = [row for row in rows if row.category == category and getattr(row, key) == "Y"]
            complete = sum(1 for row in applicable if row.status == "Complete")
            denominator = len(applicable)
            expected[category][label] = (
                "—" if denominator == 0 else f"{complete}/{denominator} ({pct(complete, denominator)})"
            )
    return expected


def totals_reconcile(
    rows: list[Row],
    headline: dict[str, dict],
    categories: dict[str, dict[str, str]],
) -> list[str]:
    errors = []
    expected = expected_headline(rows)
    for label, values in expected.items():
        actual = headline.get(label)
        if actual is None:
            errors.append(f"headline: missing {label}")
            continue
        for field, value in values.items():
            if actual.get(field) != value:
                errors.append(f"headline {label}: {field} is {actual.get(field)!r}, expected {value!r}")
    expected_cats = expected_categories(rows)
    for category, values in expected_cats.items():
        actual = categories.get(category)
        if actual is None:
            errors.append(f"category summary: missing {category}")
            continue
        for label, value in values.items():
            if actual.get(label) != value:
                errors.append(f"category {category} / {label}: {actual.get(label)!r}, expected {value!r}")
    return errors


def partial_zero_credit(rows: list[Row], headline: dict[str, dict]) -> list[str]:
    errors = []
    expected = expected_headline(rows)
    for label, values in expected.items():
        actual = headline.get(label)
        if actual and actual.get("coverage") != values["coverage"]:
            errors.append(f"partial zero credit: {label} coverage is not Complete/denominator")
    return errors


def in_scope_separate(rows: list[Row], headline: dict[str, dict]) -> list[str]:
    errors = []
    expected = expected_headline(rows)
    for label, key in BASELINES:
        in_label = f"{label}, in scope"
        actual = headline.get(in_label)
        if actual is None:
            errors.append(f"in-scope: missing {in_label}")
            continue
        expected_values = expected[in_label]
        if actual.get("denominator") != expected_values["denominator"]:
            errors.append(f"in-scope {label}: denominator is not the in-scope row count")
        if actual.get("excluded") != expected_values["excluded"]:
            errors.append(f"in-scope {label}: excluded non-goal count is wrong")
    return errors


def zero_denominator(rows: list[Row], headline: dict[str, dict]) -> list[str]:
    errors = []
    expected = expected_headline(rows)
    for label, values in expected.items():
        actual = headline.get(label)
        if actual is None:
            continue
        if values["denominator"] == 0 and actual.get("coverage") != "—":
            errors.append(f"{label}: zero denominator is not displayed as —")
        if values["denominator"] > 0 and actual.get("coverage") == "—":
            errors.append(f"{label}: non-zero denominator is displayed as —")
    return errors


def rounding_rule() -> list[str]:
    errors = []
    for complete, denominator, expected in ((1, 3, "33.3%"), (2, 3, "66.7%"), (1, 2, "50.0%"), (0, 0, "—")):
        actual = pct(complete, denominator)
        if actual != expected:
            errors.append(f"rounding: {complete}/{denominator} is {actual}, expected {expected}")
    return errors


def example_source_valid(source: str, sources: int, gaps_text: str) -> bool:
    value = source.strip().strip("`").strip()
    match = re.fullmatch(r"(?:src|conflict):(\d+)", value)
    if match:
        return 1 <= int(match.group(1)) <= sources
    if value.startswith("non-goal:"):
        return "ROADMAP.md:" in value or "feasibility:" in value
    if re.fullmatch(r"#\d+", value):
        return True
    if re.fullmatch(r"[A-Z]{2,5}-\d+", value):
        return re.search(rf"\b{re.escape(value)}\b", gaps_text) is not None
    if value.startswith("follow-up:"):
        return bool(value[len("follow-up:"):].strip())
    return False


def examples_present(
    examples: list[dict[str, str]], rows: list[Row], sources: int, gaps_text: str
) -> list[str]:
    errors = []
    ids = {row.id for row in rows}
    kinds = {example["kind"].lower() for example in examples}
    for kind in EXAMPLE_KINDS:
        if kind.lower() not in kinds:
            errors.append(f"examples: missing {kind}")
    for example in examples:
        if example["row"] not in ids:
            errors.append(f"examples: {example['kind']} names unknown row {example['row']}")
        source = example["source"].strip()
        if not source:
            errors.append(f"examples: {example['kind']} has no source")
        elif not example_source_valid(source, sources, gaps_text):
            errors.append(f"examples: {example['kind']} has an invalid source: {source}")
    return errors


def largest_gaps_present(text: str, rows: list[Row]) -> list[str]:
    errors = []
    ids = {row.id for row in rows}
    named = re.findall(r"\bP-[A-Z]+-\d+\b", text)
    if not named:
        errors.append("largest gaps: no tracker row is named")
    for row_id in named:
        if row_id not in ids:
            errors.append(f"largest gaps: names unknown row {row_id}")
    if not re.search(r"#\d+|follow-up:|[A-Z]{2,5}-\d+", text):
        errors.append("largest gaps: no tracking source is named")
    return errors


def baseline_history(
    history: list[dict[str, str]], assessed: str, commit: str, headline: dict[str, dict]
) -> list[str]:
    errors = []
    if not history:
        return ["baseline history: table is empty"]
    if history[0].get("date") != assessed:
        errors.append("baseline history: first row is not the current assessment date")
    if history[0].get("commit") != commit:
        errors.append("baseline history: first row is not the current Pictura commit")
    for label, field in (("CS6 Standard", "cs6s"), ("CS6 Extended", "cs6e"), ("CC 28.0", "cc")):
        values = headline.get(label)
        if values is None:
            errors.append(f"baseline history: headline is missing {label}")
            continue
        expected = f"{values['complete']}/{values['denominator']} ({values['coverage']})"
        if history[0].get(field) != expected:
            errors.append(
                f"baseline history: {field} is {history[0].get(field)!r}, expected {expected!r}"
            )
    if "28.0" not in history[0].get("cc_version", ""):
        errors.append("baseline history: CC version is not 28.0")
    assessments = [(row.get("date"), row.get("commit")) for row in history]
    if len(assessments) != len(set(assessments)):
        errors.append("baseline history: duplicate date and Pictura commit")
    return errors


def gate_wiring_present(verify_fast: str, guards: str) -> list[str]:
    errors = []
    if "== parity ==" not in verify_fast:
        errors.append("verify-fast.sh has no docs-only parity section")
    if "check-parity.py" not in verify_fast:
        errors.append("verify-fast.sh does not run check-parity.py")
    if "start_check parity parity_check" not in verify_fast:
        errors.append("verify-fast.sh has no parallel parity check")
    if "check-parity.py" not in guards:
        errors.append("guards workflow does not run check-parity.py")
    return errors


def check(root: Path = ROOT) -> list[str]:
    text = (root / "PARITY.md").read_text(encoding="utf-8")
    sections = parse_sections(text)
    header = {
        key: header_value(text, key)
        for key in ("Assessed", "Pictura commit", "CS6 baseline", "CC baseline")
    }
    rows, errors = parse_feature_rows(text)
    errors += baseline_header(header, section_body(sections, "Baselines"))
    errors += links_present(
        (root / "README.md").read_text(encoding="utf-8"),
        (root / "ROADMAP.md").read_text(encoding="utf-8"),
    )
    errors += parity_disclaimer(text)
    errors += duplicate_ids_rejected(rows)
    gaps_text = (root / "docs/dev/doc-code-parity-gaps.md").read_text(encoding="utf-8")
    sources = parse_sources(text)
    for row in rows:
        errors += na_requires_reason(row)
        errors += evidence_paths_exist(row, root)
        errors += check_field_valid(row)
        errors += non_complete_tracks(row, gaps_text)
        errors += conflict_applicability(row)
        errors += token_errors(row, sources)
    headline = parse_headline(text)
    categories = parse_categories(text)
    errors += totals_reconcile(rows, headline, categories)
    errors += partial_zero_credit(rows, headline)
    errors += in_scope_separate(rows, headline)
    errors += zero_denominator(rows, headline)
    errors += rounding_rule()
    errors += examples_present(parse_examples(text), rows, sources, gaps_text)
    errors += largest_gaps_present(section_body(sections, "Largest remaining gaps"), rows)
    errors += baseline_history(
        parse_history(text), header["Assessed"], header["Pictura commit"], headline
    )
    errors += gate_wiring_present(
        (root / "scripts/verify-fast.sh").read_text(encoding="utf-8"),
        (root / ".github/workflows/guards.yml").read_text(encoding="utf-8"),
    )
    return errors


def self_check() -> list[str]:
    rows = [
        Row("P-T-01", "Complete", "Y", "Y", "Y", "Complete", "in-scope", "crates/pictura-core/src", "none", "", "—", "Test"),
        Row("P-T-02", "Partial", "Y", "Y", "Y", "Partial", "in-scope", "crates/pictura-core/src", "none", "", "TOOL-1", "Test"),
        Row("P-T-03", "CC only", "N/A", "N/A", "Y", "Missing", "in-scope", "—", "none", "post-CS6:src:1", "follow-up:x", "Test"),
        Row("P-T-04", "Retired", "Y", "Y", "N/A", "Missing", "non-goal", "—", "none", "retired:src:1", "non-goal:ROADMAP.md:1", "Test"),
    ]
    errors: list[str] = []
    headline = expected_headline(rows)
    categories = expected_categories(rows)
    errors += totals_reconcile(rows, headline, categories)
    bad = copy.deepcopy(headline)
    bad["CS6 Standard"]["coverage"] = "100.0%"
    if not totals_reconcile(rows, bad, categories):
        errors.append("self-check: a wrong total was accepted")
    errors += partial_zero_credit(rows, headline)
    errors += in_scope_separate(rows, headline)
    errors += zero_denominator(rows, headline)
    zero_rows = [Row("P-T-05", "Standard absent", "N/A", "Y", "Y", "Complete", "in-scope", "crates/pictura-core/src", "none", "edition:Extended-only", "—", "Test")]
    zero_headline = expected_headline(zero_rows)
    errors += zero_denominator(zero_rows, zero_headline)
    bad_zero = copy.deepcopy(zero_headline)
    bad_zero["CS6 Standard"]["coverage"] = "0.0%"
    if not zero_denominator(zero_rows, bad_zero):
        errors.append("self-check: a zero denominator was accepted as 0.0%")
    if not duplicate_ids_rejected([rows[0], rows[0]]):
        errors.append("self-check: duplicate IDs were accepted")
    if not na_requires_reason(Row("P-T-06", "N/A", "N/A", "Y", "Y", "Missing", "in-scope", "—", "none", "", "TOOL-1", "Test")):
        errors.append("self-check: an unexplained N/A was accepted")
    if evidence_paths_exist(Row("P-T-07", "Evidence", "Y", "Y", "Y", "Complete", "in-scope", "no/such/path", "none", "", "—", "Test")) == []:
        errors.append("self-check: a missing evidence path was accepted")
    if check_field_valid(Row("P-T-08", "Check", "Y", "Y", "Y", "Complete", "in-scope", "crates/pictura-core/src", "automated:bad::missing", "", "—", "Test")) == []:
        errors.append("self-check: an unresolved automated check was accepted")
    if check_field_valid(Row("P-T-09", "Check", "Y", "Y", "Y", "Complete", "in-scope", "crates/pictura-core/src", "automated:check-parity::rounding_rule", "", "—", "Test")):
        errors.append("self-check: a resolved automated check was rejected")
    gaps = "| TOOL-1 | gap |"
    if non_complete_tracks(Row("P-T-10", "Track", "Y", "Y", "Y", "Missing", "in-scope", "—", "none", "", "—", "Test"), gaps) == []:
        errors.append("self-check: an untracked in-scope row was accepted")
    if non_complete_tracks(rows[1], gaps):
        errors.append("self-check: a gap ID was rejected")
    if non_complete_tracks(rows[3], gaps):
        errors.append("self-check: a non-goal citation was rejected")
    if conflict_applicability(Row("P-T-11", "Conflict", "Y", "Y", "Y", "Unknown", "in-scope", "crates/pictura-core/src", "none", "conflict:1", "—", "Test")):
        errors.append("self-check: a valid conflict row was rejected")
    if conflict_applicability(Row("P-T-12", "Conflict", "Y", "Y", "N/A", "Unknown", "in-scope", "crates/pictura-core/src", "none", "conflict:1", "—", "Test")) == []:
        errors.append("self-check: a conflict row with N/A was accepted")
    if links_present("[PARITY.md](PARITY.md)", "[PARITY.md](PARITY.md)"):
        errors.append("self-check: a valid links pair was rejected")
    if not links_present("", ""):
        errors.append("self-check: missing links were accepted")
    if not links_present("[PARITY.md](docs/PARITY.md)", "[PARITY.md](PARITY.md)"):
        errors.append("self-check: a broken link target was accepted")
    header = {
        "Assessed": "2026-11-20",
        "Pictura commit": "98c0b292",
        "CS6 baseline": "Photoshop CS6 13.0",
        "CC baseline": "Photoshop 28.0 (2026)",
    }
    if baseline_header(header, "src:4"):
        errors.append("self-check: a valid baseline header was rejected")
    if not baseline_header({}, ""):
        errors.append("self-check: an empty baseline header was accepted")
    if not parity_disclaimer(""):
        errors.append("self-check: a missing disclaimer was accepted")
    examples = [{"kind": kind, "row": "P-T-01", "source": "src:1"} for kind in EXAMPLE_KINDS]
    if examples_present(examples, rows, 12, gaps):
        errors.append("self-check: a valid examples table was rejected")
    if not examples_present(examples[:-1], rows, 12, gaps):
        errors.append("self-check: a missing example was accepted")
    bad_source = copy.deepcopy(examples)
    bad_source[0]["source"] = "src:99"
    if not examples_present(bad_source, rows, 12, gaps):
        errors.append("self-check: an out-of-range example source was accepted")
    if not largest_gaps_present("", rows):
        errors.append("self-check: an empty largest-gaps section was accepted")
    if largest_gaps_present("P-T-01 #1", rows):
        errors.append("self-check: a valid largest-gaps section was rejected")
    if not largest_gaps_present("P-FAKE-99 #1", rows):
        errors.append("self-check: an unknown largest-gap row was accepted")
    history_row = {
        "date": "2026-11-20",
        "commit": "98c0b292",
        "cs6s": (
            f"{headline['CS6 Standard']['complete']}/{headline['CS6 Standard']['denominator']}"
            f" ({headline['CS6 Standard']['coverage']})"
        ),
        "cs6e": (
            f"{headline['CS6 Extended']['complete']}/{headline['CS6 Extended']['denominator']}"
            f" ({headline['CS6 Extended']['coverage']})"
        ),
        "cc_version": "28.0",
        "cc": (
            f"{headline['CC 28.0']['complete']}/{headline['CC 28.0']['denominator']}"
            f" ({headline['CC 28.0']['coverage']})"
        ),
    }
    if not baseline_history([], "2026-11-20", "98c0b292", headline):
        errors.append("self-check: an empty baseline history was accepted")
    if baseline_history([history_row], "2026-11-20", "98c0b292", headline):
        errors.append("self-check: a valid baseline history was rejected")
    same_day = dict(history_row, commit="89c8ba95")
    if baseline_history([history_row, same_day], "2026-11-20", "98c0b292", headline):
        errors.append("self-check: distinct assessments on the same date were rejected")
    if not baseline_history([history_row, history_row], "2026-11-20", "98c0b292", headline):
        errors.append("self-check: a duplicate assessment was accepted")
    bad_history = dict(history_row)
    bad_history["cs6s"] = "0/0 (0.0%)"
    if not baseline_history([bad_history], "2026-11-20", "98c0b292", headline):
        errors.append("self-check: a wrong history count was accepted")
    if not gate_wiring_present("", ""):
        errors.append("self-check: missing gate wiring was accepted")
    good_wiring = 'echo "== parity =="\ncheck-parity.py\nstart_check parity parity_check'
    if gate_wiring_present(good_wiring, "check-parity.py"):
        errors.append("self-check: valid gate wiring was rejected")
    malformed = (
        "### Test\n\n"
        "| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |\n"
        "|---|---|---|---|---|---|---|---|---|---|---|\n"
        "| P-T-99 | short |\n"
    )
    _, parse_errors = parse_feature_rows(malformed)
    if not parse_errors:
        errors.append("self-check: a malformed feature row was accepted")
    errors += rounding_rule()
    return errors


def main() -> int:
    if "--self-check" in sys.argv[1:]:
        errors = self_check()
        if errors:
            print("\n".join(errors))
            print("check-parity self-check: FAIL")
            return 1
        print("check-parity self-check: PASS")
        return 0
    try:
        errors = check()
    except FileNotFoundError as error:
        print(f"check-parity: {error}")
        return 1
    if errors:
        print("\n".join(errors))
        print("check-parity: FAILED")
        return 1
    print("check-parity: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
