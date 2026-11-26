# Proposal

## Why

The Qt Test layer builds one executable and one empty AUTOMOC unit per suite. Fetched `origin/master` has 270 suite sources, so a small test added to the tree also adds a full target and link. Two long suites remain unsharded even though their per-function reset already makes them safe to split.

## What Changes

- Replace the per-suite executables with a single CMake registry that builds one `tst_driver_<subsystem>` executable per subsystem. Each driver dispatches `--suite <name>` to the existing suite entry point.
- Keep one process per CTest invocation, the existing JUnit path pattern, per-suite environment, timeout, and failure return value. Self-spawning suites pass their suite name to the driver.
- Add `tst_quick_mask` and `tst_document_navigate` to `PICTURA_QT_TEST_SHARDS` and use the existing `shardedMain` path. Quick Mask keeps its 300-second timeout on every share.
- Keep every test body and assertion. Replace only the entry-point invocation, plus the command arguments in tests that restart their own executable.
- Add dispatch checks to `tst_test_support` for argument forwarding, suite-name `argv[0]`, return codes, path normalization, missing or malformed selectors, unknown suites, and duplicate registrations.
- Fail CMake configuration when a standalone suite source is missing from the registry or a suite appears in more than one subsystem list. Included implementation sources stay explicit and outside the suite registry.
- Record cold and warm build measurements and the generated CTest diff against `origin/master`; see `tasks.md` and `test-plan.md` for the numbers.

Consolidation preserves the existing registrations. The requested sharding replaces the two unsharded entries with four shares each, so the total CTest count rises by six. No existing entry outside those two changes name, JUnit path, environment, or timeout.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `verification/qt-test-harness`: replace the one-executable-per-source rule with registry-driven subsystem drivers while retaining one process and one suite identity per CTest invocation.

## Impact

- CMake: `crates/pictura-app/cpp/tests/CMakeLists.txt` becomes the sole suite registry and maps each suite to its driver target. A configure-time completeness guard compares the registry with the suite sources.
- C++: `qt_test_support.h/.cpp` gain the registry and dispatcher. Suite sources gain a registration wrapper around their existing main; custom mains and self-spawn call sites change only where dispatch requires it.
- Tests: `tst_test_support` gains driver-dispatch coverage. No existing assertion is removed or weakened.
- Build: grouped targets reduce AUTOMOC and link work without changing the shared precompiled-header setup. The three sources with executable-path definitions skip the reused PCH explicitly. No dependency or allowlist entry is added.
- Tooling: CTest-based scripts remain compatible because they use entry names, JUnit paths, and `command[0]`. No direct per-suite binary invocation was found in scripts or CI.
- Application behavior, self-test tokens, and feature parity are unchanged.
