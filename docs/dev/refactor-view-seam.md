# Refactor — view seam adapters (round 3)

- **Status:** open (issue #276). Companion docs issue #281 lands this brief
  before any code.
- **Type:** C++ façade namespaces over the existing cxx-qt bridge, plus a few
  hybrid typed Rust calls. No algorithm moves, no new dependency, no second
  bridge.
- **Contract:** observable behavior, self-test tokens, and exit codes stay
  identical. Units land independently and each leaves the build green.
- **Supersedes:** Track 4 of `docs/dev/refactor-tool-architecture.md`
  (single-bridge growth policy).
- **Change artifacts:** `openspec/changes/typed-cpp-api-adapters/`
  (capability `ui/view-seam-adapters`).

## 1. Decision

The Rust↔Qt seam is one god module: a single `#[qobject] PictureView` carries
~254 `#[qinvokable]` methods over ~46 free-function bridge modules, 142
production C++ files hold a raw `PictureView*`, and geometry crosses as
space-separated strings re-parsed at dozens of call sites. Instead of growing
that surface, C++ consumes it through **typed adapter namespaces** under
`crates/pictura-app/cpp/api/`, each registered explicitly in `CMakeLists.txt`:

| Namespace | Unit | Absorbs |
|---|---|---|
| `document_api`, `history_api` | A | open/new/save/dirty, typed `CanvasSize` for `display_level_size`, undo/redo/history navigation |
| `selection_api` | B | typed `SelectionContour` / `SelectionBounds`, selection verbs |
| `paint_api`, `transform_api` | C | stroke lifetime (`Stroke`), typed `LayerRect` / `Quad` / preview matrix, `composite_bounds` |
| `layers_api` | D | `layers_snapshot(thumb_size, entire_doc)` row model, layer ops |
| `type_api` | E | type/text façade over the 31 `type_tools` functions |

Adapters exist only where they absorb real complexity — string protocols, call
ordering, batching, or a unit façade that centralizes a multi-call sequence.
A 1:1 forwarder of a single invokable is not an adapter and is not added.
Generated `*.cxxqt.h` headers become an implementation detail included solely
by adapter translation units, except named temporary deferrals for
`frame_includes.h` lines whose unit has not landed.

## 2. Hybrid typed Rust calls

Three crossings earn a typed Rust addition; each replaces deleted surface, so
`cxxqt_object.rs` shrinks:

- `layers_snapshot(thumb_size, entire_doc) -> Vec<LayerRow>` collapses the
  Layers panel's ~20 per-field invokables per row (plus the per-effect
  `layer_style_value` probe) into one call. The return vehicle (a `qinvokable`
  returning `Vec<LayerRow>` vs a free `extern "Rust"` fn) is chosen by a spike
  before unit D; both look the same from C++.
- Typed `LayerRect`, `Quad`, `CanvasSize`, `SelectionContour`,
  `SelectionBounds`, and the 9-coefficient preview matrix cross instead of
  space-separated strings, converted once at the adapter.
- Paint ordering (`begin_paint` → `flush_present` → `end_paint`, the mid-stroke
  `image()` ban) moves from folklore into `paint_api::Stroke`: a tool-lifetime
  object constructed on press, `finish()`/`cancel()` on release/Esc, destructor
  closes exactly once. Mid-stroke present queues `flush_present` and never
  calls `image()`.

Once no production or test caller remains, superseded string invokables
(`display_level_size`, `layer_rect`, `transform_quad`,
`transform_preview_matrix`, `selection_contour`, `selection_bounds`,
`composite_bounds`, and the 29 `layer_row_*` entry points) are deleted.

## 3. Migration order

One branch (`refactor/276-typed-cpp-api-adapters`), one PR; units land
independently in this order, each with its consumer `rg` inventory, deletions,
and tests:

1. docs (#281, this file) and the `Vec<LayerRow>` spike
2. unit A — document + history
3. unit B — selection
4. unit C — paint + transform
5. unit D — layers + snapshot (`tst_layers_snapshot`)
6. unit E — type
7. integration: full verification bar, allowlist shrink, named-deferral list

## 4. Boundary statement

Pure logic and data stay in Rust; work that is inherently about the Qt
boundary — widgets, dialog layout, include policy, stroke lifetimes tied to
tool objects — stays in C++. An adapter is C++ at the boundary: it may hold
`PictureView&` inside its `.cpp` and expose typed structs in its header, but
it forwards to the bridge and owns no document algorithm. New engine
operations still grow the Rust bridge; adapters stop *consumer* sprawl, they
do not replace the bridge. This is the policy Track 4 stated as "only a
genuinely new engine operation grows the bridge", kept under a new home.

## 5. Gate

Same bar as every refactor round (`docs/dev/refactor-tool-architecture.md`
§5), plus `ctest -R '^tst_'` including `tst_layers_snapshot` and
`scripts/check-selftest-budget.sh` (no new `ST_BEGIN` sites).
## 6. `type_api` after #403: deepened, not folded

Unit E landed as a near 1:1 forwarder: 48 of its 60 functions had a one-line
body, which the §1 rule rejects. Issue #403 weighed two fixes and chose to
**deepen** it.

- **Rejected: fold it back.** Deleting `type_api` would put the generated
  `type_tools*.cxxqt.h` headers back into about 35 files, against the
  exclusive-surface rule in `ui/view-seam-adapters`.
- **Chosen: deepen.** The adapter now absorbs the unit's real shapes:
  - `TypeSpec` (text, font, setting, character, paragraph) replaces the
    five-argument tuple on every preview, bounds, commit and update call;
  - `readLayer` is the only layer read (the per-field getters are gone);
  - `EditSession` owns re-editing: begin, then exactly one commit or cancel,
    and a session dropped open cancels;
  - `WarpSession` owns the Warp Text preview the same way;
  - `characterStyles` / `paragraphStyles` read each style library in one
    call. The count/name/font/setting index getters left the bridge.
- **The one header exception.** `api/type_api.h` still includes the generated
  type headers, because its value types are the bridge's shared setting
  structs. No other production file includes them, other adapters included
  (`layers_api.cpp` asks `type_api::isWarped`). The Rust test
  `type_bridge_headers_stay_in_adapter` enforces this.

Change artifacts: `openspec/changes/rust-single-source-bundle/`.

## 7. Round 4: domain membranes (#1160)

The tree drifted back to 82 bridge modules and 356 C++ files including a
generated header. A literal single bridge module is blocked by cxx-qt 0.10's
one-module-per-file rule and the 1,200-line cap; one umbrella header would make
every translation unit parse every generated header. The seam now uses eight
hand-written membranes under `cpp/api/`:

| Membrane | Domain |
|---|---|
| `view_bridge.h` | the `PictureView` QObject |
| `document_bridge.h` | open/save/recovery, history, guides, new/size/mode |
| `layers_bridge.h` | layer stack, masks, styles, paths, shapes, channels |
| `paint_bridge.h` | paint tools, healing, magnetic, annotations |
| `filters_bridge.h` | filter tools, adjustments, HDR, apply/match/replace |
| `selection_bridge.h` | selection, refine, colour range |
| `transform_bridge.h` | transform, warp, crop, align |
| `engine_bridge.h` | actions, clipboard, colour management, export, decode, MCP |

Every C++ file that reaches the bridge includes a membrane, never a `*.cxxqt.h`
directly.
`api/type_api.h` remains the type domain's generated-header surface. The bridges
that exposed one or two Rust functions were folded into neighbours; six bridges
landed afterwards, so the tree holds 74 (the budget the ratchet records). The
bridge stamp is still global: any bridge edit recompiles every
membrane consumer; the membranes bound the per-translation-unit include surface,
not the rebuild fan-out. Tests:
`pictura_app::build_config::{bridge_headers_confined_to_api,
bridge_membranes_are_include_only, no_universal_bridge_membrane,
no_small_bridge_files, bridge_count_within_budget,
adapter_and_membrane_headers_are_registered,
generated_include_scan_rejects_violation}`.

Change artifacts: `openspec/changes/archive/2026-11-04-bridge-membranes/`.

## 8. Round 5: checked-in registration (#1187)

Round 4 deferred "verb registration is in one place" to #1187. The deferral hid
a distinction: adding a **verb** to an existing bridge was already one edit, and
adding a **bridge** was three (the source, a line in `build.rs`'s `.files([...])`
list, the membrane). Two checked-in generated artifacts replace both
bookkeeping steps:

| Artifact | Holds | Read by |
|---|---|---|
| `crates/pictura-app/bridge_sources.txt` | the 74 bridge paths, sorted | `build.rs`'s `.files(...)` |
| `crates/pictura-app/src/build_config/verb_manifest.tsv` | 1,465 verbs: kind, C++ name, Rust name, owning `cpp/api/` header, bridge header | humans, and the drift test |

`build.rs` no longer names a bridge source inline. It reads the list and emits
one `rerun-if-changed` for it; the bridge files stay watched through
`CxxQtBuilder::files`, and nothing else under `src/` is watched, so an
implementation-body edit still does not rerun the build script. A build-time
scan of `src/` was rejected: watching the directory reruns the script on every
Rust edit (5.3 s to about 55 s of build-script time), and the script cannot skip
its own work on such a rerun because `CxxQtBuilder::build()` is the only emitter
of this crate's Qt `rustc-link-lib` directives and the `cxxqt_*` cfg.

Both artifacts are generated, never hand-edited, and both are drift-tested in the
nextest run `verify-fast` already performs:

```bash
PICTURA_WRITE_BRIDGE_SOURCES=1 cargo nextest run -p pictura-app bridge_sources
PICTURA_WRITE_VERB_MANIFEST=1   cargo nextest run -p pictura-app verb_manifest
```

Adding a bridge is now: create the source, add its generated header to a
membrane, regenerate the list. Adding a verb is one declaration edit; the
manifest drift test then names the row to bless.

Deliberately not done: generating the membranes (their grouping is editorial),
per-membrane bridge stamps (a translation unit can reach a membrane through a
header, which `OBJECT_DEPENDS` cannot express), and pruning verbs with no
caller. Tests: `pictura_app::build_config::bridge_sources::{bridge_sources_match_the_tree,
bridge_sources_are_all_real_bridges}`,
`pictura_app::build_config::verb_manifest::{verb_manifest_matches_bridge_sources,
verb_manifest_counts_every_declared_function, verb_manifest_names_owners,
verb_manifest_names_renamed_verbs}`,
`pictura_app::build_config::build_script_derives_bridge_sources`.

Change artifacts: `openspec/changes/verb-manifest-registration/`.

