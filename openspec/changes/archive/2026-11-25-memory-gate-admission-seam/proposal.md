# Proposal

## Why

"Will this edit fit?" is not one call today. Every gated command re-assembles an
ambient protocol: clear the last refusal, pick one of 17 byte estimators, call
one of two admission functions, then remember to read a thread-local refusal
flag and translate it. A caller that forgets `clear()`, picks the wrong
estimator, or reads the wrong flag is silently wrong, and nothing stops it
compiling.

That failure mode is not hypothetical. `quick_mask_document` asks the governor
without clearing, and the paint entry point returns into the Quick Mask path
before its own `clear()` runs. Quick Mask Move preview and commit likewise reject
content before clearing. A later command that fails to start for a non-memory
reason can therefore present an earlier memory refusal (`tool_brush.cpp:75`).
On the C++ side the three-way refusal decision is re-derived five times, and the
generic presenter at `memory_refusal.cpp` misses the scratch-full case that all
four bespoke ladders get right.

## What Changes

- One admission seam: a gated command supplies its declared cost and receives
  one outcome. The seam owns clearing, estimation hand-off, governor room, the
  scratch spill check, and the typed refusal reason.
- The admission outcome becomes a return value rather than a boolean, so a
  caller cannot ignore a refusal. `Admitted { spill }` folds in today's separate
  `expects_spill` question.
- The refusal reason becomes one typed value with three cases (memory,
  unreadable scratch tiles, full scratch disks), replacing the two refusal flags.
  The GUI-thread `LAST_REFUSAL` remains the record C++ reads when a boolean
  command returns before its caller can present the result. The governor's own
  refusal is only a fallback when the seam has no typed reason, so a late
  allocation failure is still presented as a memory refusal.
  The existing `edit_refused_for_*` predicates remain as derived reads:
  `edit_refused_for_memory` accepts `Memory` and `ScratchFull`, rejects
  `ScratchUnreadable`, and falls back to the governor only when no typed reason
  exists.
- **BREAKING** (internal API only): the admission verbs return the outcome
  instead of `bool`. Admissions own their reset; commands that can fail for a
  non-memory reason before admission still clear at the command boundary.
- One C++ presentation path consumes the typed reason; the four bespoke
  per-command ladders supply wording, not a second decision. Geometry commit
  presentation keeps its existing Memory/ScratchFull scope.
- Fixes the Quick Mask stale-refusal defect for paint, Move preview and Move
  commit: a successful or non-memory-refused command must not present a refusal
  recorded before it.
- Test pins (pinned host, cgroup, scratch availability) move out of the seam's
  surface into a test-only module. Inline seam tests move to
  `helpers/memory_gate_tests.rs` through the repository's `#[path]` pattern.

## Capabilities

### New Capabilities

- `document/memory-gate-admission`: the one admission seam a gated command
  calls. Owns the admission outcome, the typed refusal reason, the single
  presentation path, and the rule that refusal state never outlives the edit
  that produced it.

### Modified Capabilities

- `document/memory-governor`: the requirement that every benchmark path "report
  'not enough memory'" becomes a requirement to present the typed refusal
  reason through one path, distinguishing full scratch disks from a memory
  shortfall, and never presenting a refusal for an edit that succeeded.

## Impact

- `crates/pictura-app/src/cxxqt_object/helpers/memory_gate.rs` keeps the
  estimators, two admission verbs, typed refusal record and test pins below the
  800-line target; its tests live in `helpers/memory_gate_tests.rs`.
- Resident and spilling call sites collapse to one admission call each. Commands
  with non-memory early returns retain an explicit boundary clear.
- `helpers/geometry_admit.rs` consumes `Admission`. `helpers/save_gate.rs` keeps
  its reservation and `SaveAdmission` contract, recording scratch-full through
  the shared typed refusal.
- `crates/pictura-app/cpp/memory_refusal.{h,cpp}` gains one presenter keyed by
  the typed reason; `tool_fills.cpp`, `tool_move.cpp`, `tool_transform.cpp` and
  `image_size_dialog.cpp` keep their copy but drop their own three-way switch.
- `paint_tools/imp.rs`, `quick_mask/imp.rs` and the Move preview/commit entry
  points clear before Quick Mask can reject a command for a non-memory reason.
- `tst_memory_governor.cpp` is the end-to-end check; new cases exercise each
  command family through the seam and retain the existing scratch-full tests.
