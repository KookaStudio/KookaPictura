# Proposal

## Why

The Rust↔Qt seam is 82 `#[cxx_qt::bridge]` modules that generate 82 headers
(81 are consumed), and 348 C++ files outside `cpp/api/` (194 production, 154 Qt
tests) include one directly; 8 adapter files under `cpp/api/` also include
generated headers and stay as they are. Every caller must know which generated
header owns which verb. Issue #1160 asks for one bridge root behind the typed
adapters.

A literal single Rust module is not viable in this tree: `cxx-qt-build 0.10`
permits exactly one bridge module per file, so "one root" means a roughly
7,000-line declaration file against a 1,200-line cap and a shrink-only allowlist
(`cxxqt_object.rs` is already at 1,204). A single umbrella header is also the
wrong shape: it would make every translation unit parse every generated header,
regressing the per-translation-unit include surface. It would not change
rebuild fan-out: the single global bridge stamp already recompiles every
generated-header consumer on any bridge edit, and this change keeps that
behaviour. The seam's actual problem is that callers choose headers, not that
the headers exist.

## What Changes

- Add a small set of hand-written **domain membranes** under
  `crates/pictura-app/cpp/api/` (document, layers, paint, filters, selection,
  transform, engine, misc — the exact grouping comes from consumer
  co-inclusion, not taste). Each membrane includes only the generated headers
  its domain owns. `cpp/api/type_api.h` remains the type domain's
  generated-header surface and is the named adapter-owned exception.
- Migrate every C++ file outside `cpp/api/` — production, Qt tests, and
  `frame_includes.h` — to include the membrane(s) it uses and no generated
  `*.cxxqt.h`. Generated headers become reachable only through `cpp/api/`.
- Fold the bridges that expose at least one `extern "Rust"`/`extern "RustQt"`
  function and two or fewer into their domain neighbours, delete the standalone
  bridge files, and drop them from `build.rs`; the bridge count drops from 82 to
  69. Import-only bridges such as `impl_paint/tool_recording.rs` are exempt.
- Move the CMake bridge-stamp dependency from a scan for `pictura_app/src/` in
  each `.cpp` to a scan for either `pictura_app/src/` or an `api/*_bridge.h`
  membrane, so a bridge edit still rebuilds every consumer in the same build.
  Rebuild fan-out stays global, as it is today.
- Add a shrink-only guard: zero direct generated-header includes outside
  `cpp/api/`, bridge count not above 69 (the post-fold count), and
  `cxxqt_object.rs` not above 1,204. The file-size cap itself stays in
  `check-file-size.sh`.
- **Deferred, explicitly:** "verb registration is in one place." This is a
  deliberate narrowing of issue #1160 and needs maintainer sign-off; a
  generated verb manifest can deliver it later without merging modules. This
  change does not add it.
- No runtime behavior change. Self-test tokens, exit codes, public C++ APIs, and
  the generated ABI stay identical. The fold preserves namespaces;
  `decode_bridge::copy_bytes` is kept unnamespaced (or its four call sites are
  updated) so `::copy_bytes` does not change.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `ui/view-seam-adapters`: the generated-header confinement rule stops allowing
  named deferrals and instead requires domain membranes under `cpp/api/`;
  `cpp/api/type_api.h` remains the named type-adapter exception. The
  small-bridge rule (at least one exposed Rust/RustQt function, two or fewer)
  and a shrink-only ratchet with a post-fold budget of 69 are added.
- `meta/build-performance`: the bridge stamp dependency is attached through the
  membrane headers or a still-named generated header rather than a per-source
  string scan, preserving same-build recompilation after a bridge edit.
  Rebuild fan-out remains global.

## Impact

- Rust: fold the small bridge files under `crates/pictura-app/src/**`; the
  bridge count drops to 69. The fold also re-homes each bridge's `imp` module.
  No bridge module changes beyond moving declarations into a neighbour;
  `build.rs` `.files([...])` shrinks.
- C++: new `cpp/api/<domain>_bridge.h` membranes (no `.cpp`); include edits
  across 348 files (194 production + 154 tests); the 8 adapter files under
  `cpp/api/` do not change. `CMakeLists.txt` registers the membranes and
  rewrites the stamp dependency.
- Guards: new tests in `build_config.rs`, run by verify-fast's nextest step; no
  new script. The existing bridge shape tests keep passing, and
  `type_api.h` remains the only type generated-header surface.
- Tests: Qt test sources change includes only; no assertion changes.
- Dependencies: none.
