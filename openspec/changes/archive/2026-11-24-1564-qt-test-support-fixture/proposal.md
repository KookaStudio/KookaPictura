# Proposal

## Why

`crates/pictura-app/cpp/tests/qt_test_support.h` is a header-only fixture
included by 250 of the 269 Qt Test translation units, and it includes `frame.h`,
whose 19-file, 5,017-line closure every suite then re-parses. The fixture's own
280 lines are likewise parsed 250 times for no shared code. Two couplings ride
along: nine QObject headers never include their own `moc_<header>.cpp`, so their
meta-object code lands in the central `mocs_compilation.cpp` against
`meta/build-performance`'s `bp_moc_includes`; and two hand-written bridge edges
compile twice, by `cc` and by CMake. The issue's acceptance asks for zero test
translation units including `frame.h`, which this change cannot reach: 241 suites
name `PicturaMainWindow` and 226 already reach `frame.h` through another header.
The measured outcome is 250 → 242, and the rest is #1451's window facade.

## What Changes

- Split `qt_test_support.h` into a declaration header and one
  `qt_test_support.cpp`. A new STATIC library `pictura_test_support` owns the
  `.cpp` and every `tst_*` target links it, so the fixture's bodies compile once
  instead of 250 times. `makeMainWindow` is declared against a forward-declared
  `PicturaMainWindow`; the header no longer includes `frame.h`. The
  `shardedMain` template stays in the header, because a template cannot be
  compiled out of line.
- Add `#include "frame.h"` to the suites that name the window and do not already
  reach it through another header, so the dependency is explicit instead of
  accidental.
- Append `#include "moc_<header>.cpp"` to the nine `.cpp` files that own a
  QObject header's meta-object code, as the other 175 already do. Their hidden
  entries leave `mocs_compilation.cpp`.
- Give the two doubly-compiled bridge edges one owner. Both are reached from Rust
  production code, and `cargo test -p pictura-app` links the Rust test binary
  from the same static library, so `cc` keeps the compile and the two CMake
  entries are removed. No build-time scan of `src/` is added; `bridge_sources.txt`
  stays the build script's input.
- Extend `scripts/check-shell-seams.py` with two mechanical checks: no
  `tests/qt_test_support.h` may include `frame.h`, and every QObject header with
  a same-named `.cpp` must have that `.cpp` include its own moc. The second is
  the enforcement `bp_moc_includes` has been missing.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `meta/build-performance`: the moc-ownership requirement gains a mechanical
  check, and a new requirement fixes one compiling owner per hand-written bridge
  edge. The `bp_mold*`, `bp_cheap_tu`, `bp_pch*` and profile requirements are
  untouched.
- `meta/cmake-source-fragments`: the Rust-built exclusion list in "Every C++
  translation unit appears exactly once" grows from one source to three.
- `verification/qt-test-harness`: the harness requirement names the compiled
  support library each suite links and the declaration-only fixture header.

## Impact

- C++: `crates/pictura-app/cpp/tests/qt_test_support.h` shrinks to declarations;
  new `crates/pictura-app/cpp/tests/qt_test_support.cpp`; new
  `pictura_test_support` STATIC target in
  `crates/pictura-app/cpp/tests/CMakeLists.txt` with `AUTOMOC OFF` (a moc scan
  of the support unit would otherwise generate a second `frame.h` meta-object).
  The existing `pictura_test_pch` OBJECT library and its `REUSE_FROM` wiring are
  unchanged.
- Nine shell `.cpp` files gain their own moc include; no target, source list, or
  compile flag changes as a result.
- CMake: `cmake/sources/documents.cmake` loses `decode_image.cpp` and
  `encode_image.cpp`; `scripts/check-cmake-source-fragments.py` treats all three
  Rust-built edges as excluded rather than one.
- Scripts: `scripts/check-shell-seams.py` gains `check_moc_ownership` and
  `check_test_fixture_seam` plus self-test coverage.
- Specs: deltas under `meta/build-performance`, `meta/cmake-source-fragments`,
  and `verification/qt-test-harness`.
- Dependencies: none. No runtime behaviour changes; every suite keeps its
  assertions and the self-test token stream is untouched.
