# Tasks

## 1. Checker and fixtures first

- [x] 1.1 Write `scripts/check-parity.py` with the parsers, validators, and the self-check functions named in `test-plan.md` (`links_present`, `baseline_header`, `non_complete_tracks`, `partial_zero_credit`, `in_scope_separate`, `totals_reconcile`, `rounding_rule`, `evidence_paths_exist`, `check_field_valid`, `parity_disclaimer`, `examples_present`, `duplicate_ids_rejected`, `na_requires_reason`, `gate_wiring_present`, `largest_gaps_present`, `baseline_history`, `conflict_applicability`, `zero_denominator`); verify `python3 scripts/check-parity.py --self-check` passes on its fixtures. Keep the script under 800 lines: `scripts/check-file-size.sh` only scans `crates/`, so this is a design rule, not an enforced gate.
- [x] 1.2 Add the fixture cases for arithmetic drift, duplicate IDs, a missing evidence path, an unexplained N/A, a missing or unresolved tracking link, missing examples without row/source, partial zero credit, in-scope counting, a zero denominator, a missing baseline-history row, a largest-gaps omission, and a conflict note on an N/A cell; verify each fixture is rejected with a named failure.

## 2. Frozen baselines and inventory

- [x] 2.1 Freeze the CS6 Standard and Extended inventories from the CS6 contract and the Adobe CS6 Help-derived menu, tool, panel, and format lists; write the baseline section of `PARITY.md` with versions, editions, assessment date, Pictura commit, the Standard/Extended delta, and the numbered Sources list; verify every category names its source list and every Extended-only row explains its Standard `N/A`.
- [x] 2.2 Research and freeze the CC 28.0 delta from Adobe's November 2026 release notes and help; record the owner-chosen 28.0 baseline with the Adobe source naming "November 2026 (version 28.0)" and the 27.11.1 LTS; add the retired capabilities (legacy 3D first) and the named CC-only additions (Artboards first) with `src:<n>` Notes references; verify every CC-only or retired row cites an Adobe source in Notes and every `N/A` carries an `edition:`, `post-CS6:`, or `retired:` reason.
- [x] 2.3 Add the Files & interchange, Tools & painting, Shell/panels/preferences, and Automation rows at the documented granularity; cover the expanded source units (File-menu/metadata/print/web-export; brush engine, brush dynamics, and presets); verify tool rows match the CS6 toolbox slots (71, including the ten Extended 3D tools), panel rows match the CS6 panel list, and format rows cover each supported format once.
- [x] 2.4 Add the Layers, Selections, Adjustments & filters, Type & vector, and Color rows; cover the expanded units (Image > Adjustments' 23 rows/25 commands, the Filter menu's submenus and stand-alone filters per `docs/02-ui-ux/menus.md:242-279`, and type on path/warp text/type styles/font handling); use family rows with `members:<n>/<n>` in Notes for filters, blend modes, and layer styles; verify no dialog control or option is a row.
- [x] 2.5 Fill status, scope, evidence, check, notes, and tracking for the measurable surface; leave unreconciled rows `Unknown` with a resolved in-scope tracking link; verify no `Complete` row lacks an existing evidence path, no non-`Complete` in-scope row lacks a gap-register row, issue, or `follow-up:` note, and every `Scope: non-goal` row cites its ROADMAP or feasibility non-goal entry.

## 3. Totals, examples, and conflicts

- [x] 3.1 Add the headline table (Complete/Partial/Missing/Unknown, denominator, coverage, excluded non-goals) for the three baselines and the in-scope rows; add the category table and the baseline-history table (date, Pictura commit, CS6 Standard/Extended counts, CC version, CC counts); verify `python3 scripts/check-parity.py` recomputes both totals and accepts a zero denominator.
- [x] 3.2 Add the six worked examples — complete workflow, partial implementation, unknown assessment, intentional non-goal, CC-only capability, and retired capability — each naming an existing row ID and source; verify each resolves, not merely that six kinds exist.
- [x] 3.3 Reconcile the measurement conflict across every source: correct `ROADMAP.md:457` to the Measurement Log panel only, remove the measurement engine and its Measurement Log panel from `ROADMAP.md:695-696` "Not planned" (leave DICOM), narrow `docs/dev/doc-code-parity-gaps.md:107` PAN-2 to the panel, narrow `docs/00-overview/feasibility-and-non-goals.md:74` and its Qt6 mapping at line 118 to DICOM, and correct `WF-11`. Mark measurement scale/presets/exporters `Complete` (Extended) with #635; Measurement Log panel + Record Measurements `Missing`, `in-scope`, #510; DICOM `Missing`, `non-goal`, Extended. Verify tracker, ROADMAP, feasibility, and gap register agree.
- [x] 3.4 Record every known source conflict (including Lighting Effects) with both citations and `Unknown` where the evidence does not settle it; a disputed presence is `Y` + `Unknown` + `conflict:<src>`, never `N/A`; verify no conflicted row is `Complete` and the checker rejects `N/A` with a conflict note.
- [x] 3.5 Add the largest-remaining-gaps section naming the largest full-product gaps, the categories or rows driving them, and their tracking sources; verify `largest_gaps_present` accepts it.

## 4. Links, stale claims, and update rule

- [x] 4.1 Link `PARITY.md` from `README.md` and `ROADMAP.md`; verify both links resolve from the repository root.
- [x] 4.2 Link the canonical gap register from `PARITY.md` and add the aggregate pointer to `docs/dev/doc-code-parity-gaps.md`; verify the tracker does not duplicate a gap row.
- [x] 4.3 Correct the README filter-family claim, the `ROADMAP.md:30` At-a-glance Filters row, and the ROADMAP “21 built” tool sentence; verify the filter claims agree with the roadmap's shipped entries and the filter catalogue, and the tool count with the tool catalogue.
- [x] 4.4 Write the assessment date, Pictura commit, re-freeze trigger, stable-ID rule, and same-PR update rule into `PARITY.md`; verify the update rule names `ROADMAP.md` and `PARITY.md` together.
- [x] 4.5 Update `openspec/config.yaml`'s Coverage rule to include `<script>::<fn>` alongside `tst_<suite>::<case>` and `<crate>::<fn>`; verify `python3 scripts/check-test-plan.py --self-test` still passes.

## 5. Gates, validation, and archive

- [x] 5.1 Wire `python3 scripts/check-parity.py --self-check` and `python3 scripts/check-parity.py` into BOTH the docs-only fast path and the parallel `start_check` batch of `scripts/verify-fast.sh`, and into the guards CI workflow; verify both verify-fast paths and the guards workflow run them and a deliberately broken total fails.
- [x] 5.2 Update `docs/dev/STATE.md` with the tracker location and the assessed baselines; verify the commit carries `TASK-ALLOWS-DOCS` for the `docs/` changes.
- [x] 5.3 Run `openspec validate --all --strict`, `python3 scripts/check-parity.py --self-check`, `python3 scripts/check-parity.py`, and `python3 scripts/check-test-plan.py`; verify all pass before review.
- [x] 5.4 Archive the change with `openspec archive quantified-parity-tracker` and verify `bash scripts/check-openspec-archived.sh` passes.
