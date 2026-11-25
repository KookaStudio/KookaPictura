# Design

## Context

See `proposal.md` for motivation and `specs/document/memory-gate-admission` and
`specs/document/memory-governor` for the behavior contract.

The current seam is `crates/pictura-app/src/cxxqt_object/helpers/memory_gate.rs`
(721 LOC, 32 exported functions). It exports four kinds of thing:

- **17 byte estimators** — pure functions from a document/bounds to bytes
  (`new_document_bytes`, `fill_cost`, `rotate_cost`, `fork_bytes`, …); 4 more
  live beside the transform code in `impl_transform/group.rs` and
  `impl_transform/session/preview.rs`.
- **4 admission/spill functions** — `admit`, `admit_fresh`, `admit_spilling`,
  `expects_spill`.
- **8 protocol bits** — `clear`, `refuse_for_scratch`, `refuse_scratch_full`,
  `scratch_available`, `take_unreadable_reads`, `edit_refused_for_memory`,
  `edit_refused_for_scratch`, `edit_refused_for_scratch_full`.
- **3 test pins** — `set_available_memory_for_test`, `set_cgroup_for_test`,
  `set_scratch_available_for_test`.

The protocol bits are the problem. They carry one fact ("was this edit refused,
and why") from the gate to the presenter through two refusal booleans
(`SCRATCH_REFUSED`, `SCRATCH_FULL`) plus the governor's `REFUSAL`. The other two
thread-locals are not refusal state and stay: `SCRATCH_AVAILABLE` holds the
scratch free-bytes pin, and `UNREADABLE_SEEN` is the read counter
`scratch_rollback.rs:19` diffs. 26 call sites call `clear()` (24 non-test, across
11 non-test files), 12 call `admit_fresh`, 8 call `admit_spilling`, and 23
non-test C++ reads ask the predicates (24 textual hits; the other is the doc
comment at `tools.h:196`).

Constraints that shape the approach:

- The gate is GUI-thread only, and the thread-locals are per-thread, so a
  refusal crosses to C++ only on the same thread. That is why they exist at all
  rather than a plain field.
- Jobs run on workers, but every gate is evaluated on the GUI thread before the
  worker spawns (`start_document_job` is called after the gate at
  `paint_tools/fills/imp.rs`), so a value returned to the caller still reaches
  the presenting thread.
- The save path already carries its reason across the worker boundary as a
  `NativeSaveResult { code, admission }` struct, mapped in C++ by
  `nativeAdmissionMessage` (`import_save_save.cpp:60`). That is the precedent to
  copy, not a second mechanism to invent.
- cxx-qt bridges already carry Rust enums both ways (`PasteKind`,
  `VectorPlaceResult`, returned by `vector_place_result`).

## Goals / Non-Goals

**Goals:**

- One call per gated command: cost in, outcome out.
- The compiler, not discipline, is what prevents "forgot to reset", "asked the
  wrong cost", and "read the wrong flag".
- One typed refusal reason with three cases, presented through one path.
- Test pins stop counting against the seam's surface.
- The existing `tst_memory_governor.cpp` and refusal tests keep passing
  unchanged.

**Non-Goals:**

- Re-deriving or re-fitting the 17 byte estimators. Their formulas and their
  measured-peak guarantees are `document/memory-governor`'s and stay untouched.
- Moving the governor itself (`pictura-core::memory`). The seam sits above it.
- Reworking the save path's `SaveAdmission` or the open path's
  `OPEN_REFUSED_FOR_MEMORY`. Both already carry a typed reason; they consume the
  new type where their reason is the same three cases and keep their own where
  it is not.
- Changing any message text. Only the decision that selects a message moves.
- Per-command refusal localization beyond what `tst_memory_governor.cpp` already
  pins.

## Decisions

### Decision 1: The outcome is a return value, with a GUI-thread refusal record for presentation

Gated commands return `Admission`. The seam also records the last typed reason
in the GUI-thread `LAST_REFUSAL` cell because C++ often receives only a boolean
and presents after the Rust call returns.

```rust
enum Refusal { Memory, ScratchUnreadable, ScratchFull }

enum Admission {
    Admitted { spill: bool },
    Refused(Refusal),
}

fn admit_resident(bytes: u64) -> Admission;              // was admit / admit_fresh
fn admit_commit(cost: &CommitCost) -> Admission;         // was admit_spilling + expects_spill
```

**Why over "record it on the view, keep `bool` returns":** the recorded-state
version is a smaller diff but leaves the exact hole the issue names — a caller
can ignore the return and the compiler says nothing. The return value makes the
admission decision explicit; `LAST_REFUSAL` is only the same-thread presentation
record for callers that cannot hold that return value.

`Admitted { spill }` replaces `expects_spill`. A spilling transform reads `spill`
from the same admission result in `impl_transform/session.rs`, so one call
replaces two.

**Alternative rejected:** an out-parameter (`-> bool, &mut Option<Refusal>`).
Compiles, but nothing forces the caller to read the out-parameter either — same
hole with more syntax.

### Decision 2: The refusal reason becomes the one value that crosses to C++

`Refusal` is declared in the cxx bridge as a `#[repr(u8)]` enum, so C++ receives
a real enum rather than an `i32` with magic values.

The seam stores one `Option<Refusal>` per GUI thread, not per view. Every
present-day predicate derives from it, with one explicit fallback for an
allocation that failed after admission:

```rust
fn edit_refused_for_memory() -> bool {
    match last() {
        Some(Refusal::Memory | Refusal::ScratchFull) => true,
        Some(Refusal::ScratchUnreadable) => false,
        None => governor_refusal(),
    }
}
fn edit_refused_for_scratch() -> bool { last() == Some(Refusal::ScratchUnreadable) }
fn edit_refused_for_scratch_full() -> bool { last() == Some(Refusal::ScratchFull) }
```

`ScratchFull` remains a memory refusal for existing predicate consumers, while
`ScratchUnreadable` remains distinct. The governor's refusal is consulted only
when the seam has no typed reason, so it cannot overrule a recorded scratch
reason.

**Why the predicates survive rather than being deleted:** `tst_memory_governor.cpp`
asks them after a refused call and must pass unchanged, and a handful of C++
sites present after a job lands, where there is no return value left to hold.
Keeping them costs three derived reads and buys a single source of truth. What
disappears is the two independent refusal booleans, not the question "was it
refused".

The four bespoke ladders stop *deriving* the reason, and the generic presenter
takes the `Refusal` value directly. Geometry commit presentation uses the same
typed path but retains its existing Memory/ScratchFull scope.

**Why not an `i32` code like the save path's `admission` field:** the save path
needed an `int` because it crosses a `DocumentJobResult` struct that predates
this work. New code has no such constraint, and a cxx enum is checked at
compile time on both sides.

### Decision 3: "No stale refusal" is structural at admission and explicit at early returns

Every admission resets the last refusal first. A command that can fail for a
non-memory reason before admission clears at its command boundary, because C++
may consult the predicate unconditionally after a false return. `undo` is the
clearest case: `clear(); if (!can_undo() || !restore_fits()) { return false; }`.
The same shape holds at `geometry_admit`'s `sides_fit` check, native save's
preconditions, and the Quick Mask Move siblings.

The original defect crosses three entry points: `quick_mask_document` asked the
governor without clearing; Quick Mask paint returned before the normal paint
path's clear; and Move preview/commit checked `quick_mask_content_allowed`
before clearing. `quick_mask_document` now uses `admit_resident`, while
`begin_quick_mask_paint_with_tip`, `begin_move_preview` and `commit_move` clear
before any non-memory early return.

The one legitimate late refusal — `scratch_rollback.rs`, which refuses *after*
an edit commits — records `Refusal::ScratchUnreadable` through the seam because
it is a verdict about a finished edit, not a gate.

### Decision 4: One presentation path, wording stays per command

`memory_refusal.{h,cpp}` keeps its message builders and gains one presenter that
switches on `Refusal`:

```cpp
QString refusalMessage(Refusal reason, const QString& notEnough,
                       const QString& scratchFull, const QString& unreadable);
```

The four bespoke ladders (`fillMemoryRefusalMessage`,
`moveMemoryRefusalMessage`, `transformMemoryRefusalMessage`,
`imageSizeBudgetMessage`) keep their call sites and their copy, and stop
containing the three-way decision. This closes a live bug: `reportMemoryRefusal`
tests `edit_refused_for_scratch()` and `edit_refused_for_memory()` but never
scratch-full, and a non-native (raster) save already records
`Refusal::ScratchFull` through `save_gate.rs::reserve_save_checked` ->
`refuse_scratch_full`, then reaches `reportMemoryRefusal` from
`import_save_save.cpp:366` and `:660`. A PNG/TIFF/EPS save with full scratch
disks is therefore presented today as "not enough memory". One presentation
path fixes it.

### Decision 5: Test pins move out of the seam's surface

`set_available_memory_for_test`, `set_cgroup_for_test`,
`set_scratch_available_for_test` move to a test-only module. They are how a Qt
suite pins the host, not part of the contract, and 3 of 32 exports being test
hooks obscures the seam's real size. The bridge declarations stay (the Qt suites
call them by name) but forward to the test module.

### Decision 6: The 17 byte estimators keep their names and their generality

Estimators stay free functions with the same signatures. They are cost
knowledge, not protocol, and they are what `document/memory-governor`'s
measured-peak requirement pins. Coupling them to the admission verbs — an
estimator that gates itself — would hide the cost from the tests that check it
covers the benchmark peak.

## Risks / Trade-offs

- **The diff is large and mechanical** (26 `clear()` sites across 11 non-test
  files; 26 non-test files reference `memory_gate::`; 23 C++ reads) →
  Land it in the order in `tasks.md`: seam first with the old verbs as thin
  adapters, then migrate call sites family by family, then delete the adapters
  in task 6.2.
  Every intermediate step builds and passes `tst_memory_governor.cpp`.

- **`Admitted { spill }` couples a spill prediction to the admission result** →
  The prediction already exists (`expects_spill`) and callers already pair the
  two; folding them cannot make a caller ask less often than it does now.

- **An admitted edit clears a refusal that a caller might still need** → Only
  one path needs a refusal after admittance: the rollback, which deliberately
  records its own after the commit. Everything else presents within the same
  command, before anything else gates.

- **C++ sites that gate a boolean and then present later** → These exist (a tool
  press gates, the commit presents). The reason rides the GUI-thread
  `LAST_REFUSAL` cell across the bridge call that returned the boolean; it is
  not per-view storage and is not read from a worker thread.

- **A reason recorded on one thread and presented on another** → Cannot happen
  today: every gate is on the GUI thread and workers only run post-gate engines.
  The design keeps the value on the GUI-thread path, which preserves that.

- **Removing `clear()` as a universal caller obligation breaks a Qt suite** →
  Admissions own their reset, while non-memory early-return commands keep an
  explicit boundary clear. The predicates survive as derived reads (Decision 2),
  so existing C++ tests and presenters keep compiling.

## Migration Plan

1. Introduce `Refusal`, `Admission`, `admit_resident`, `admit_commit`; keep
   `clear`/`admit`/`admit_spilling`/`expects_spill` as thin adapters.
2. Route `quick_mask_document` through `admit_resident`, and clear Quick Mask
   paint plus Move preview/commit before their non-memory early returns; confirm
   the stale-refusal cases fail before and pass after.
3. Migrate call sites by family (history, layers, paint, transform and geometry);
   audit save/open and retain their existing reservation/result contracts. The
   adapters stay until task 6.2 deletes them.
4. Add the typed `Refusal` bridge value alongside the derived predicates; update
   the C++ presenter, geometry commit path and four ladders.
5. Move the test pins to the test-only module.
6. Keep every existing refusal test and add one case per command family through
   the real admission seams; move the inline seam tests to `memory_gate_tests.rs`.

Rollback is per family: adapters stay until task 6.2, so a family is reverted by
restoring its old call sites while the adapters still exist, without touching
the seam.

## Open Questions

None. The two decisions that could have changed scope — typed return versus
recorded state, and whether the Quick Mask fix ships here — were settled before
planning: typed return, and the fix ships in this change.
