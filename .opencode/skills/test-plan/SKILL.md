---
name: test-plan
description: Write or update the test-plan.md of an OpenSpec change, map every spec scenario to an automated test or a justified manual check, and write the missing tests. Use when the user says "test plan", "manual test plan", "automate the test plan", or before implementing an OpenSpec change (AGENTS.md workflow step 3).
license: MIT
compatibility: Run from the repository root.
metadata:
  author: KookaPictura
  version: "1.0"
---

# Test plan

Input: an OpenSpec change name. If none is given, use the change touched on the
current branch (`git diff --name-only $(git merge-base HEAD master) -- openspec/changes`);
if several or none, ask.

## 1. Collect the scenarios

Read `openspec/changes/<name>/specs/**/spec.md`, `proposal.md`, and `design.md`.
Each `#### Scenario:` is one row, keyed by its trailing `[tag]`, or by its title
when untagged.

## 2. Find existing coverage

Search before writing anything new:

- GUI: `crates/pictura-app/cpp/tests/tst_*.cpp` slots (`grep -n '^\s*void .*();'`).
- Engine: `#[test] fn` under `crates/<crate>/`.

A test covers a row only if it asserts the scenario's THEN, not just exercises
the code path. Read the body.

## 3. Write `openspec/changes/<name>/test-plan.md`

```markdown
# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lcs_example` | What a tester does | What they see | `tst_suite::caseName`, `crate_name::fn_name` |
| Untagged title | ... | ... | manual: <why it cannot be automated> |
```

- Coverage is the last column: backticked `tst_<suite>::<case>` / `<crate>::<fn>`
  references, or `manual: <reason>`.
- Extra rows beyond the scenarios are welcome for manual-only checks (artwork,
  real window-manager focus, CS6 parity without an oracle).
- `manual:` is for what offscreen Qt Test or a std test cannot observe, not for
  "not written yet".

## 4. Automate the gaps

For each row without a test, add one following `docs/dev/testing-conventions.md`:
Qt Test case in the nearest `tst_*.cpp` (new GUI checks never go in
`runSelfTest()`), or a std `#[test]` in the owning crate. Follow the suite's
fixture pattern (`pictura::test::makeMainWindow`, `newDocument`,
`canvas->mousePressed/mouseMoved/mouseReleased`, `QApplication::sendEvent` for
keys). A new `tst_*.cpp` must be added to `crates/pictura-app/cpp/tests/CMakeLists.txt`.
Respect file-size caps (AGENTS.md rule 9).

## 5. Check

```bash
python3 scripts/check-test-plan.py
```

Build and run the tests only at the end (AGENTS.md workflow step 5), e.g.
`ctest --test-dir build -R '^tst_<suite>$' --output-on-failure`. Copy the manual
rows into the PR body as a reviewer checklist.
