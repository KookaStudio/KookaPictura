# Test plan

The artefacts under this change are prose, so every row is a review-time check.
Each reason names the automated gate that covers the same ground, so a reviewer
does not have to repeat a mechanical check by hand.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dc_roadmap_shipped` | Read each ticked item in `ROADMAP.md` "What works today" and "Tools" | Every ticked item names a path in the tree, and the path exists and backs the claim | manual: a prose claim cannot be a unit test; `scripts/check-doc-claims.py` verifies the referenced paths exist, and the reviewer confirms each path backs its claim |
| `dc_roadmap_open` | Read each unticked item in `ROADMAP.md` "Open work" | Every unticked item names at least one `#NNN` issue, and each issue resolves to a row in `docs/dev/doc-code-parity-gaps.md` or to a prose-tracked issue in its §F | manual: the issue-to-row mapping is a judgement; `scripts/check-doc-claims.py` enforces that the `#NNN` reference is present |
| `dc_readme_bullet` | Read each bullet in `README.md` "Features" | Every bullet names a path in the tree, and the path exists and backs the claim | manual: a prose claim cannot be a unit test; `scripts/check-doc-claims.py` verifies the referenced paths exist |
| `dc_corpus_banner` | Read the status banner of each file under `docs/` | No banner asserts the repository contains no code, and each points at `docs/dev/doc-code-parity-gaps.md` | manual: the corpus is prose; `scripts/check-doc-claims.py` fails the build when a claim reappears |
| `dc_corpus_entry` | Read `docs/README.md` | The entry point describes the corpus as the behavioural contract and names where the implementation lives; it does not claim the repository has no code, no build files, or no dependencies | manual: the corpus is prose; `scripts/check-doc-claims.py` covers the no-code claim |
| `dc_register_index` | Take an unticked `ROADMAP.md` item and follow its issue into `docs/dev/doc-code-parity-gaps.md` | The issue appears in the register's Issue column, and the register's status matches what the roadmap says about the gap | manual: the mapping is a judgement; the register's Issue column is the single index the guard points at |
| `dc_nongoal_shipped` | Take a feature listed under `ROADMAP.md` "Not planned" and search the tree for its implementation | No implementation contradicts the non-goal; the Count tool is not listed as a non-goal, because `crates/pictura-app/cpp/tool_count.cpp` ships it | manual: an absence check is a judgement; the reviewer greps the tree for the named feature |
| `dc_nongoal_absent` | Take a feature listed under `ROADMAP.md` "Not planned" that the register records as a non-goal | The register agrees, and the feature's absence is deliberate rather than a gap with no issue | manual: an absence check is a judgement; the register's §E lists the confirmed non-goals |
