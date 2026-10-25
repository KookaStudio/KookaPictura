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
