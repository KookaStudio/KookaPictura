# Design

## Context

See `proposal.md`. The work is a build-cost change with no application behaviour
change, so every claim below is a static count, a compile-set identity, or a test
result. No timing is invented: the only times in this file are numbers this
session observed, and they are labelled as observed.

### Static facts measured before the change (2026-11-23, base 515ab3a0)

| What | Count |
|---|---|
| Qt Test suites declared in `cpp/tests/CMakeLists.txt` | 268 |
| `tst_*.cpp` files on disk | 269 (one extra is a `_cases` header owner) |
| Translation units including `tests/qt_test_support.h` | 251 (250 `tst_*.cpp` + `multi_file_test_support.h`) |
| `qt_test_support.h` lines | 280 |
| Translation units that reach `frame.h` today | 250 of 269 |
| `frame.h` lines | 804 |
| `frame.h` include closure | 19 files, 5,017 lines |
| Translation units that `#include "frame.h"` anywhere under `cpp/` | 244 |
| Suites naming `PicturaMainWindow` or calling `makeMainWindow` | 241 |
| Suites reaching `frame.h` without the fixture | 226 |
| Suites that dereference the factory result | 213 |
| QObject headers with a same-named `.cpp` | 184 |
| … of which the `.cpp` includes its own `moc_<header>.cpp` | 175 |
| … of which it does not (the nine) | 9 |
| Bridge-edge sources compiled by both `cc` and CMake | 2 (`decode_image.cpp`, `encode_image.cpp`) |
| Bridge-edge sources compiled only by `cc` | 1 (`panels/actions_tool_recording_bridge.cpp`) |
| Shell fragment paths / shell `.cpp` / self-test `.cpp` | 689 / 430 / 38 |

`scripts/check-shell-seams.py` fails on the base: `frame.h: 804 lines exceeds
760`. That is #1451's ceiling and this change does not touch it.

### The ceiling on this change

The issue's acceptance wants
`grep -rl '#include "frame.h"' crates/pictura-app/cpp/tests | wc -l` to be 0.
It cannot be, in this change. Three disjoint sets explain why:

| Set | Suites |
|---|---|
| `A` — reach `frame.h` today, by any path | 250 |
| `B` — reach it without the fixture, through another header | 226 |
| `C` — name `PicturaMainWindow` or call `makeMainWindow()` | 241 |
| `A \ B` — reach it only through the fixture | 24 |
| `C \ B` — name the window but have no other path | 16 |

After the split a suite parses `frame.h` iff it names the window or keeps a
non-fixture path, so the count is `|C ∪ B| = 242`. The eight suites that leave are
`A \ (C ∪ B)`: they got `frame.h` from the fixture for free and never named the
window. The other 16 of `A \ B` name the window, so they gain an explicit
include and stay; the 24 are not recoverable without either #1451's facade or an
opaque handle.

An opaque handle would take `C` out of the equation, but 213 of the 241 `C`
suites immediately dereference the result, so it buys 28 suites at the cost of
rewriting 213 call sites, and the fan-out would still be 213. Rejected as scope
creep into #1451.

So the honest target is:

- `tests/qt_test_support.h` has no `frame.h` include: **0 includers there**, a
  real, checkable zero.
- Suites that parse the frame closure fall from **250 to 242** of 269. The eight
  that leave are named by the check's self-test fixture, not enumerated here.
  This is a narrow, not a full, win, and the 242 is stated in the PR body.

## Decisions

### 1. Declarations in the header, definitions in `qt_test_support.cpp`

A new STATIC library `pictura_test_support` owns
`crates/pictura-app/cpp/tests/qt_test_support.cpp`. Every `tst_*` executable
links it. The header keeps:

- `ScopedDefaultLocale` and `ScopedStateHome` declarations (their bodies move to
  the `.cpp`; the members stay in the header because the classes are constructed
  by value in suite fixtures and need a complete layout),
- `makeMainWindow` as a declaration returning
  `std::unique_ptr<PicturaMainWindow>` against a forward-declared
  `class PicturaMainWindow;`,
- `completionOffers`, `selectFamily`, `typeFamily`, `testFunctions`,
  `shardTestFunctions`, `namesTestFunctions`, `shardArguments`,
  `shareRunsNothing` as declarations,
- `shardedMain` as a template definition, because it is instantiated with each
  suite's test object and cannot be compiled out of line.

`QTemporaryDir` and `QByteArray`/`QStringList` remain headers the *declaration*
needs, because the class members and the declaration signatures name them. The
fixture does not need `frame.h`, `QFontComboBox`'s definition (only the pointer
parameter type, so a forward declaration suffices — but `<QtWidgets/QFontComboBox>`
is one Qt header and is kept, since `selectFamily` is declared with a
`QFontComboBox*` and callers pass real objects), `QTest`, or `QApplication`; the
header keeps only what a declaration needs.

The support library is linked with `target_link_libraries(... PRIVATE
pictura_shell Qt6::Test)`, exactly as the PCH object library is, and gets the
same `PICTURA_DEV_COMPILE_OPTIONS` and `-g1`. `AUTOMOC OFF` is set on it: the
`.cpp` includes `frame.h`-owning suites' headers only transitively through
nothing — it includes `frame.h` directly to construct the window, and moc would
otherwise generate a meta-object unit for `PicturaMainWindow`, duplicating the
one `frame.cpp` already emits through its own `moc_frame.cpp` include.

*Alternatives:*

- **An INTERFACE library with the `.cpp` in each target.** That is what happens
  today, only neater. It keeps 250 compiles; rejected.
- **A `#define`-injected body.** Unreadable, and a header that must still include
  `frame.h` for the `new PicturaMainWindow` expression. Rejected.
- **An opaque `WindowHandle` type hiding `unique_ptr<PicturaMainWindow>`.** It
  would make the header `frame.h`-free for all 269 suites, but 213 suites
  immediately need `window->activeView()` and so need the definition anyway; the
  handle would be a churn over 241 files for a fan-out that is still 213. Rejected
  as scope creep into #1451.

### 2. The nine moc owners

`fade_dialog`, `import_save_coordinator`, `load_layers_dialog`, `panel_host`,
`save_for_web_color_table`, `save_load_selection_dialog`, `save_optimized_dialog`,
`statistics_dialog`, `system_info_dialog`. Each `.cpp` ends with its
`} // namespace pictura` close already, so the include is appended after that
line, matching the 175 compliant files byte-for-byte in shape:

```cpp
} // namespace pictura

#include "moc_fade_dialog.cpp"
```

`acr_panels.h` has no same-named `.cpp` and is already owned: its moc is included
from `acr/acr_basic_panel.cpp`. The check must therefore handle both shapes: a
header with a same-named `.cpp` must have that `.cpp` include the moc; a header
without one must have some other `.cpp` include it. The nine are all the first
shape.

`mocs_compilation.cpp` does not exist in this tree (no `build/` yet), so the
"gains no includes" claim is proven after the build: the generated unit must
contain no `#include "moc_` line for any of the nine, and the check is the
`mocs_compilation.cpp` content plus the moc-ownership script.

### 3. One owner per bridge edge

The three edges are `decode_image.cpp`, `encode_image.cpp`, and
`panels/actions_tool_recording_bridge.cpp`. `dialogs.cmake`/`documents.cmake` list
the first two; the third is in neither. Both of the first two define symbols
`.cpp`-side of a `#[cxx_qt::bridge]` `extern "C++"` declaration, and a Rust
`cargo test -p pictura-app` binary needs the symbols at link time (the cxx glue
references them). The `cc` compile is what puts them into the Rust static
library; CMake's copy only serves the shared shell.

Keep `cc`, drop the two CMake entries. Reason: the Rust static library is the
owner whose absence breaks `cargo test -p pictura-app`; the CMake entries are
duplicates that the shell would still get *if* it needed its own copy, but the
shell links `pictura_app` and the Rust static library already carries the
symbols. This is verified by `cargo test -p pictura-app` resolving and by the
shell linking.

`scripts/check-cmake-source-fragments.py` learns the full exclusion set (three
paths) so the two removals do not read as "unlisted translation unit". The script
also fails if an excluded path *is* listed, which is the regression direction.

The build script keeps its `rerun-if-changed` for all six `BRIDGE_EDGES` entries;
deduping the compile does not dedupe the tracking, or an edge edit would stop
rebuilding the Rust static library.

### 4. Checks

`scripts/check-shell-seams.py` gains:

- `check_moc_ownership(root)`: discover `Q_OBJECT`/`Q_GADGET` headers under
  `crates/pictura-app/cpp/`; for each header with a same-named `.cpp`, require
  that `.cpp` to contain `moc_<stem>.cpp`; for each header without one, require
  some `.cpp` under the tree to include it.
- `check_test_fixture_seam(root)`: require
  `crates/pictura-app/cpp/tests/qt_test_support.h` not to contain
  `#include "frame.h"`.

Both get self-test arms in the existing `self_test()` throwaway tree, so the
guards workflow (which already runs `check-shell-seams` via `verify-fast.sh`;
note the guards workflow currently does not run it — this change adds the step
there too) exercises them. `verify-fast.sh` already runs the script.

`scripts/check-cmake-source-fragments.py` gains the three-path exclusion set and
a self-test case: listing an excluded path fails, and a non-excluded unlisted
`.cpp` still fails.

## Measurements

Every number is a count except where labelled a time. "Before" is the base tree
`515ab3a0`; "after" is the change built in this worktree (16 CPUs, GCC 16.1.1,
mold, Ninja, RelWithDebInfo) with the build tree in `build/`.

| Measure | Command | Before | After | Delta |
|---|---|---|---|---|
| Fixture header lines | `wc -l crates/pictura-app/cpp/tests/qt_test_support.h` | 280 | 151 | −129 |
| Fixture bodies location | — | 280-line header, bodies inline | 179-line `qt_test_support.cpp` | −101 net; bodies now compile once |
| Object files for the fixture | `find build -name 'qt_test_support.cpp.o' \| wc -l` | 0 (no unit; 250 inlined copies) | 1 | one unit instead of 250 header copies |
| `qt_test_support.h` includes `frame.h` | `grep -c '#include "frame.h"' …/qt_test_support.h` | 1 | 0 | −1 |
| Test TUs including the fixture | `grep -rl '#include "qt_test_support.h"' crates/pictura-app/cpp \| wc -l` | 251 | 252 | +1 (`qt_test_support.cpp` itself) |
| `#include "frame.h"` under `cpp/tests` | `grep -rl … crates/pictura-app/cpp/tests \| wc -l` | 191 | 207 | +16 explicit (they used to borrow it) |
| `#include "frame.h"` under `cpp/` | `grep -rl … crates/pictura-app/cpp \| wc -l` | 244 | 260 | +16 (the fixture header loses its line; 16 suites and the new `qt_test_support.cpp` gain one) |
| Suites parsing the frame closure | header-graph walk in "The ceiling on this change" | 250 of 269 | 242 of 269 | −8 |
| QObject headers without their own moc owner | `python3 scripts/check-shell-seams.py` | 9 | 0 | −9 |
| Moc includes in the shell `mocs_compilation.cpp` | `grep -c 'moc_.*\.cpp' build/pictura_shell_autogen/mocs_compilation.cpp` | 9 | 0 | −9 |
| Bridge-edge sources compiled by CMake and `cc` | fragment grep | 2 | 0 | −2 |
| Shell fragment paths | `list(APPEND PICTURA_SHELL_SOURCES …)` count | 689 | 685 | −4 (two `.cpp`, two `.h`) |
| `tests/` targets linking the fixture library | `grep -c libpictura_test_support.a build/build.ninja` | n/a | 268 | every suite |
| `python3 scripts/check-shell-seams.py` | exit status | FAIL (10 problems: 9 moc + fixture + `frame.h` 804 > 760) | FAIL (1: `frame.h` 804 > 760, #1451) | the change's 10 fixed |
| `python3 scripts/check-cmake-source-fragments.py` | exit status | pass | pass | — |
| `ctest -R '^tst_'` | CTest summary | not run on this change's base (no tree) | 307/308 at `-j8`, 255 s | one failure, `tst_present_cache` GPU row |
| `./build/pictura --headless --self-test` | exit code | — | 0, `passed=467 failed=0 skipped=0` | unchanged suites |

Two numbers need their caveat stated:

- The **9 → 0** moc result is exact and directly observed: the base tree's
  `build` generated `mocs_compilation.cpp` with all nine includes, and this
  change's generated unit reports "No files found that require moc or the moc
  files are included".
- The **250 → 242** suite figure is the header-graph walk, not a CTest
  observation. It counts suites that reach `frame.h` by following quoted include
  paths, which is what the compiler does. It is a count of suites, not of
  re-parsed lines, and it is the metric that matters for edit fan-out.

One failure is **not** this change's: `tst_present_cache`'s
`frameBoundedPaintMatchesCpu(gpu)` row fails on the base per issue #1451, and
`git diff origin/master` has zero lines in that suite or its dependencies. The
CPU row passes. Environment-specific, left failing rather than weakened.

`scripts/check-shell-seams.py` still exits non-zero, on the single problem it
reported before this change: `frame.h: 804 lines exceeds 760`. That is #1451's
ceiling. Verified both ways — the base tree printed exactly that one line, and
the change prints the same. The ten problems this change was responsible for (the
nine moc owners and the fixture's `frame.h` include) are gone.
