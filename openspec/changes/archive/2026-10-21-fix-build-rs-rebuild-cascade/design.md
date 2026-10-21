# Design

## Measurements

The baseline this work started from, and the result this work reached, on a
16-core Linux box with a warm sccache, a warm Cargo target directory and
`-j12`. "Edges" is the number of Ninja edges the build ran; the steady state
is 3.

| Scenario | Before | After |
|---|---|---|
| Clean build | 384 s (818 edges) | 58.5 s (1225 edges) |
| Leaf `.cpp` touch, first build | 58.0 s (132 edges) | 3.0 s (132 edges) |
| Leaf `.cpp` touch, second build | 56.0 s (512 edges) | 0.0 s (3 edges) |
| Non-bridge `.rs` touch, first build | 73.0 s (3 edges) | — |
| Non-bridge `.rs` touch, second build | 40.0 s (512 edges) | — |
| Bridge `.rs` change, first build | 93.0 s (3 edges, **C++ stale**) | 60.0 s (506 edges, C++ rebuilt) |
| Bridge `.rs` change, second build | 59.0 s (512 edges) | 0.0 s (3 edges) |
| Cargo build, `QMAKE` switched | 145–154 s | 0.0 s |
| `ctest -R '^tst_'` at `-j4` | 136 s | 99.6 s |
| `ctest -R '^tst_'` at `-j12` | 120.5 s, 3 suites timed out | 53.2 s, no timeouts |
| `verify-fast.sh` | 511 s | see the PR body |

The two numbers that matter are the second build of a leaf `.cpp` touch
(56 s of pure waste) and the first build of a bridge change (3 edges, so the
C++ was compiled against the previous header).

The clean build's 58.5 s is 39 s of cargo edge plus 19.5 s of C++ phase: the
cargo edge starts first and every shell translation unit waits for it, so
nothing else runs until it finishes. Of the 1225 edges, 27 ran inside the
cargo edge's window and none of them was a shell translation unit.

The CTest critical path before the change named one suite:
`tst_floating_workspaces` at 82.3 s, `tst_workspaces` at 65.1 s and
`tst_import_pdf` at 55.9 s, against a 136 s total. The three suites do real
work — window and panel construction, PDF parsing — rather than sleeping, so
the lever is splitting them, not replacing waits.

The QMAKE spike (task 1.4) came out **go** on the question it asked —
Cargo's `[env]` table does reach the build-script fingerprint on toolchain
1.98 — but the value it can hold is not usable. `qt-build-utils` runs qmake
with a **cleared environment**, where a bare name is not found on the PATH:
the value has to be a path, and a path is machine-specific. The CI proved
it: its Qt is not in `/usr/bin`, and the cxx-qt build script failed with
"Could not find Qt installation: QMAKE environment variable specified as
qmake6".

So the value is not pinned in `.cargo/config.toml`. What decision 5 settles is
that **no script and no build system exports a QMAKE of its own**:

- a bare `cargo build`, `cargo clippy`, `cargo nextest` and rust-analyzer's
  flycheck have no `QMAKE` in their environment, and the cxx-qt build script
  finds qmake on the PATH itself;
- the CMake build's cargo invocation gets the path the cxx-qt module chose
  (`Qt::qmake`), which is a path every machine has.

The fingerprint is therefore stable in each build tree, which is what the
cascade needed: the bare Cargo tree records no `QMAKE` and the CMake tree
records the same path on every run. A different qmake still reruns the build
script: `QMAKE=/usr/bin/qmake6 cargo build -p pictura_app` takes 207 s.

`scripts/check-build-lock.sh` takes 17.5 s of wall time and 0.9 s of CPU.
Every second of it is a deliberate sleep: the queue case holds the lock for
2 s, the timeout case waits out a 1 s timeout and then waits for its holder,
the zero-timeout case holds for 2 s, and the reclaim and publication races
sleep 1 s each while they prove that the waiters serialize. The sleeps are
the assertions — a shorter hold would prove less — and the check runs in CI
only, so the time is not on anybody's critical path. It is not changed.

## Context
The constraints that shape the approach, all verified against the tree this
change starts from:

- `cxx-qt-build 0.10` defaults `crate_include_root` to `Some("")`
  (`src/lib.rs:413`), so `build()` calls
  `utils::best_effort_copy_headers(crate_dir, …, emit_rerun_if_changed = true)`
  (`src/lib.rs:1283-1305`), which emits `cargo::rerun-if-changed=` the crate
  directory (`src/utils.rs:24`). Cargo watches that directory recursively, so
  every file under `crates/pictura-app/` — every shell translation unit — is
  an input to the build script.
- `cxx-qt-build` writes every generated header with `File::create`
  (`src/lib.rs:205`, `245`), so the header's modification time advances on
  every build-script run even when the bytes are identical.
- The generated headers reach the C++ through
  `${CXX_QT_EXPORT_DIR}/crates/pictura_app/include`, and every shell
  translation unit that includes one names it in its depfile, so Ninja
  tracks it by modification time.
- The C++ compile edges carry an **order-only** dependency on the cargo edge
  (`cmake_object_order_depends_target_pictura_shell: phony || cargo-build_pictura_app`),
  so cargo runs first — but Ninja computes the dirty state of the whole graph
  in a pre-pass, before any edge runs. A header rewritten during the build
  cannot be seen by that build.
- `cc 1.6` emits no `cargo::rerun-if-changed` for the sources it compiles.
- 267 of the shell translation units include a generated header; 140 do not.
- `qt-build-utils 0.10` prints `cargo::rerun-if-env-changed=QMAKE`
  unconditionally (`src/installation/qmake.rs:96`), and `cxx_qt_import_crate`
  always exports `QMAKE=<absolute path>` into the cargo invocation
  (`CxxQt.cmake:108`).
- Each Qt Test executable writes its own JUnit report from inside the test
  process (`-o <file>,junitxml`), so a suite that never starts writes none.

## Goals / Non-Goals

**Goals:**

- Make a leaf C++ edit cost one translation unit and nothing else.
- Make a bridge edit cost one build, not two, and never compile the C++
  against a stale header.
- Make the report and the gates say what actually happened.
- Keep every lever independently revertible, and record the measurement that
  decided it.

**Non-Goals:**

- Changing the application binary, its sources, or its dependencies.
- C++ modules, unity builds, nightly Rust, or a new build dependency.
- A new caching layer over Ninja; the fix works with the graph as it stands.
- Splitting the cxx-qt bridge into extra crates; revisit after this.

## Decisions

### 1. The build script tracks the bridge, not the crate

`.crate_include_root(None)` drops the crate-directory directive. Nothing
includes `pictura_app/cpp/…` — the only `pictura_app/…` includes in the tree
are the *generated* `src/*.cxxqt.h` — and the hand-written bridge headers
resolve through `cc.include("cpp")`, so disabling the crate include root
removes a copy nothing reads.

Dropping the directive also drops the only change tracking `cc` does not
provide, so the build script emits `cargo::rerun-if-changed` for the three
hand-written bridge edges (`decode_image`, `render_text`, `encode_image`),
header and source each.

*Alternative considered*: keep the crate root and narrow it with a
`rerun-if-changed` list. Cargo has no "exclude" directive, so the crate root
would have to go anyway.

### 2. Generated headers are written only when their content changed

`CxxQtBuilder::build()` writes into `$CXX_QT_EXPORT_DIR`. The build script
snapshots the exported header tree before the call and compares it after,
restoring the old modification time for every file whose bytes are
unchanged (`std::fs::File::set_times`, stable since 1.75 — no new
dependency). The exported tree holds only the generated headers once
decision 1 lands, so the snapshot is a few megabytes at most.

This makes the exported header's modification time a faithful "the generated
interface changed" signal. Without it, a no-op build-script rerun — a
`QMAKE` change, a touch of a bridge file — would move the timestamp and cost
a full C++ rebuild.

*Alternative considered*: patching `cxx-qt-build` upstream. Out of reach
here; the snapshot is contained in our own build script.

### 3. The bridge stamp makes the rebuild land in the same build

Ninja's dirty pre-pass means the C++ cannot see a header that changes during
the build. The signal that *is* available before the build is the bridge
source's own modification time — but Ninja will not look at the bridge
sources, only at the generated header.

So the build system adds the missing link:

```
  CMake configure                       Ninja plan                  C++ compile
  -------------                         ----------                  -----------
  CMAKE_CONFIGURE_DEPENDS ─┐
  on the bridge .rs files  │
                          ├─▶ re-configure when a bridge file changed
  stamp = SHA256(bridge)  ┘        │
                                    ├─▶ stamp mtime advanced
                                    │        │
                                    │        └─▶ C++ objects depend on the stamp
                                    │              (OBJECT_DEPENDS)
                                    │              → the C++ is dirty AT PLAN TIME
                                    ▼
                          cargo edge runs (order-only) → header regenerated
                                    ▼
                          the C++ compiles against the fresh header
```

The stamp is written **content-aware**: a re-configure that finds the same
bridge content leaves the file, and its timestamp, alone. Without that,
`verify-full.sh` — which re-configures on every run — would dirty the whole
C++ tree every time.

The C++ objects take the stamp through CMake's `OBJECT_DEPENDS`, one
property set per source list, so Ninja sees it as an explicit input of each
compile edge.

*Alternatives considered*:

- *`restat` on the cargo edge.* Ninja's `restat` decides whether
  **downstream** edges of that edge need rebuilding; it does not add edges
  to a plan that was already computed, so it cannot make the C++ rebuild in
  the same build.
- *Copying the exported headers into a second include directory with
  `copy_if_different`.* This needs the generated file list, which does not
  exist until the first cargo build has run, so the first configure would
  have to re-configure — and the copy's own input churns on every cargo run,
  which would dirty the C++ on every build.
- *Running cargo before `cmake --build`.* Correct, but it splits the build
  into two commands and a developer who types `cmake --build build` still
  gets the two-build behaviour. The stamp fixes the same problem without
  changing the developer's command.

### 4. The clean build overlaps the Rust build

The shell's 140 translation units that include no generated header do not
need the Rust build. Today they wait for it, because the order-only
dependency comes from the *target-level* link on `pictura_app` and applies to
every source of the target.

CMake's per-source `OBJECT_ORDER_DEPENDS` can express the finer dependency,
but the target-level one cannot be removed while `pictura_app` is linked into
the shell. The shell is therefore split into two targets — one holding the
translation units that include a generated header, one holding the rest —
and only the first waits for the Rust build. The split is a pure move of
source lists.

*Alternative considered*: leaving the split out and reporting the clean
build as unchanged. Rejected: the 400 s serial cargo edge with an idle
compiler is the single largest item in the clean build, and 140 idle
translation units are the cheapest way to fill it.

### 5. One QMAKE, from one source

The fingerprint compares the value of `QMAKE` in **cargo's own process
environment**, and the CMake-driven cargo invocation exports an absolute path
that a bare cargo run does not. The two therefore disagree, and the cxx-qt
build script reruns.

The fix is to make the process environment carry the same value in every
flow. Cargo's `[env]` table in `.cargo/config.toml` is the one mechanism
that applies to every cargo invocation regardless of who started it, and
Nothing else exports a `QMAKE`: the local verification scripts export none,
so the value does not depend on which tool started the build. An explicit
`QMAKE` in the environment is honoured and reruns the build script, as it
should.

Whether Cargo's `[env]` reaches the fingerprint is **not** a given:
rust-lang/cargo#10358 reports that `cargo:rerun-if-env-changed` does not
react to values set through the `env` configuration. So the task list makes
this a **spike with a recorded outcome**, and the fallback — CMake honours
`$ENV{QMAKE}` when it is set, and the developer exports it once — is written
down before the implementation starts rather than discovered halfway.

The value the fingerprint compares has to be a **path**, because
`qt-build-utils` runs qmake with a cleared environment, where a bare name is
not found on the PATH. A path is machine-specific, so it cannot be pinned in
a committed file: the CMake build uses the one the cxx-qt module chose
(`Qt::qmake`, which `find_package(Qt6)` resolved), and a bare Cargo
invocation exports none and lets the build script find qmake on the PATH.
Both supported distributions ship `qmake6`
(`scripts/install-deps-fedora.sh:133` names the Fedora package).

*Alternative considered*: `QMAKE=<absolute path>` from a wrapper script in
the repository. The path differs per worktree, so the fingerprint would
differ per worktree — harmless, but it also breaks when the repository moves.
The bare name is one value for every machine.

### 6. A missing report is a failure

Each suite writes its own JUnit file, so a suite that never starts leaves
nothing behind and the reporter cannot tell "not registered" from "not run".
The reporter is therefore given the list of suites CTest registered and
compares it against the reports that were produced; a registered suite with
no report is a failure, named in the report.

*Alternative considered*: parsing CTest's own output for `***Not Run***`.
CTest already returns non-zero in that case, so the script failed — but the
report said "0 failed", which is the dishonest part. Comparing the registered
suites against the produced reports is exact and does not depend on CTest's
output format.

### 7. The self-test layer runs concurrently

The two self-test invocations are independent processes against the same
binary, and the suites isolate their state through `XDG_STATE_HOME`. The
self-test sources contain no `QStandardPaths` or configuration-directory
writes, so two concurrent invocations do not share state. They run
concurrently with each other and with the CTest layer.

### 8. The caching check configures once by default

The default mode configured the project three times to assert three launcher
rules. The launcher rules that matter for the local loop — sccache on `PATH`
— can be asserted from one configuration. The other two cases (an explicit
override, and sccache absent) need their own configuration and move to a
`--full` variant that CI runs.

### 9. The heavy translation units are measured, not assumed

The three slowest Qt Test suites do not use fixed wall-clock sleeps; they
pump the event loop and do real work — window and panel construction, and
PDF parsing. The lever that pays is therefore **splitting** them into more
registered entries so the layer's critical path is not one suite, not
replacing sleeps with retry helpers.

The split is a share of the suite's test functions, chosen by the
`PICTURA_TEST_SHARD` environment variable (`<index>/<count>`). The suite
enumerates its own test functions — the private slots without parameters,
minus the lifecycle hooks and the data functions, which is the rule QTest
itself uses — and hands the share's names to QTest. A suite registered
without the variable runs every function, so the suites stay runnable by
hand.

Only suites whose test functions are independent of each other are split. A
suite that shares its window between the test functions keeps one entry:
`tst_tilt_shift` builds on the state the previous function left, and its
shards failed when they ran apart.

### 10. The clean build does not overlap the Rust build with the C++ one

The clean build is 58.5 s: 39 s of cargo edge, then 19.5 s of C++ phase. The
cargo edge starts first and every shell translation unit waits for it, so
nothing else runs until it finishes.

Splitting the shell into a target that waits for the Rust build and a target
that does not would move the 123 translation units that include no generated
header — about 55 s of CPU — into the cargo edge's window. The C++ phase
would shrink by roughly 5 s and the clean build with it: 58.5 s to about
54 s, 7.7%.

That is not worth shipping. The split needs to know which translation units
include a generated header, and the answer is transitive: 145 of the 268
shell translation units reach one through a project header, 18 more than the
direct include scan finds. Deciding it at configure time needs an
include-graph walk in CMake, and a translation unit the walk puts on the
wrong side compiles against a stale generated header in an incremental build
— the very bug this change exists to remove. The build tree's own dependency
files answer the question exactly, but only after a build, which is the case
the split is meant to help.

The measurement is recorded here and the requirement is dropped from the
delta rather than shipped as a mechanism that can reintroduce the bug it
fixes.

### 11. The heavy translation units need no split

The heaviest shell translation unit is `frame_keys.cpp` at 0.51 s and the
heaviest Qt Test translation unit is `tst_brush_tip_preview.cpp` at 0.86 s.
Neither dominates a 58.5 s build, so neither is split.

The per-suite moc cost is 4.2 s over 129 compilation units, and the 138
test links cost 0.1 s each — the shared shell already made the links cheap.
Neither is worth reducing.

Split debug information does not pay either. Compiling one shell translation
unit cold takes 1.67 s with `-g` and 1.78 s with `-gsplit-dwarf`: the object
shrinks from 458 KiB to 64 KiB, but the compile is 6.6% slower, the links
have nothing to gain, and sccache does not cache the separate `.dwo` file.
The Qt Test targets already compile with `-g1`, which keeps only the line
tables.

## Risks / Trade-offs

- **[The build script writes into a third party's output directory]** → The
  snapshot is scoped to the exported header tree, and only byte-identical
  files get their timestamp restored. If a future cxx-qt changes the tree
  layout, the snapshot finds nothing and the build degrades to today's
  behaviour rather than failing.
- **[The stamp depends on a CMake re-configure]** → A bridge edit costs one
  extra configure (~2–5 s). A leaf C++ or Rust edit triggers no
  re-configure.
- **`OBJECT_DEPENDS` adds an input to every compile edge** → About 400
  extra lines in `build.ninja`; Ninja's parse time is unaffected in
  practice.
- **[The `[env]` spike may fail]** → The fallback (CMake honours
  `$ENV{QMAKE}`) is weaker: it needs a one-time export in the developer's
  shell. The outcome is recorded in this file either way.
- **[Two concurrent self-test processes]** → Guarded by the state isolation
  the suites already rely on; the first execution confirms it.
- **[A sharded suite's functions must be independent]** → A suite that
  shares state between its test functions keeps one entry. A future function
  that depends on an earlier one would fail in its shard, loudly, and the
  suite would go back to one entry.

## Migration Plan

Every step is a separate commit and separately revertible. The build script
changes ship first (decisions 1 and 2), because they are the ones that
remove work; the stamp (decision 3) ships with them, because without it the
write-if-changed change alone would leave the two-build behaviour in place.
The report and gate changes ship next — they are correctness, not speed, and
they are independent of the build changes. The clean-build and test-layer
changes ship last, each behind its measurement.

Rollback is a revert of the individual commits. The build script change
alone reverts to today's cascade; the stamp alone reverts to the two-build
behaviour; neither breaks the build.

## Open Questions

- Whether the shell split into two targets is worth the churn, or whether
  overlapping the Rust build with the *test* translation units is cheaper.
  Answered by the measurement in task group 5; the answer only chooses
  between two implementations of the same requirement.
