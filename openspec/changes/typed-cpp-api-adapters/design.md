# Design

## Context

See proposal.md for why. Constraints that shape the approach:

- One `#[cxx_qt::bridge]` `PictureView` (cxx-qt 0.10) with 254 invokables; free-fn
  bridges already exist as separate `#[cxx_qt::bridge]` modules taking
  `PictureView&`/`Pin<&mut PictureView>` (e.g. `layer_style.rs`) — the pattern is
  proven in-tree (~46 such modules, ~430 free fns).
- `cxxqt_object.rs` is 1220 LOC, allowlisted against a 1200 cap; deletions shrink it.
- CMake lists every C++ file explicitly; adapters are new `pictura_shell` sources.
- Free-function bridges are ordinary cxx codegen; `#[qinvokable]` returns go through
  cxx-qt's Qt-facing codegen. `QList_f64` already crosses; no custom shared struct
  return exists in-tree yet.
- 142 production C++ files hold `PictureView*`; control server alone has ~71
  `view->` call sites; `frame_includes.h` pulls five `*.cxxqt.h` headers used by the
  `frame_*.cpp` TUs.
- String-protocol inventory (production sites): `display_level_size` (6 files),
  `layer_rect` (properties_panel, tool_crop, tool_transform, …), `transform_quad` /
  `transform_preview_matrix` (tool_transform, image_view), `selection_contour`
  (frame, image_view), `selection_bounds` (control_server, frame_menus_image,
  info_panel), `composite_bounds` (tool_crop, tool_perspectivecrop); 36 string-
  equality assertion hits in `cpp/tests/` + `selftest*`.
- Verification bar (issue #276 + CI): both headless self-tests unchanged codes;
  `cargo nextest run --workspace`; `cargo test --workspace --doc`; `ctest -R '^tst_'`
  incl. new `tst_layers_snapshot`; `cargo fmt --all --check`;
  `cargo clippy --workspace --all-targets -- -D warnings`;
  `openspec validate --all --strict`; `scripts/check-file-size.sh`; `scripts/guard.sh`;
  `scripts/check-selftest-budget.sh` (budget 511, lower-only — no new `ST_BEGIN`).
- Companion docs: issue #281, before code (user decision). Branch
  `refactor/276-typed-cpp-api-adapters`, one PR.

## Goals / Non-Goals

**Goals:**

- Exclusive, typed C++ surface for the bridge where adapters absorb string protocols,
  batching, or ordering (or centralize a unit's multi-call sequence).
- Kill the Layers N+1 and the string-geometry re-parse matrix for migrated units.
- Make paint stroke ordering unbreakable at the adapter API for multi-event tools.
- Shrink `cxxqt_object.rs` by deleting superseded entry points; keep Track 4's spirit
  ("only a genuinely new engine operation grows the bridge") under a new policy.

**Non-Goals:**

- Splitting `PictureView` into multiple QObjects or a generic `dispatch(verb, args)`.
- Moving document algorithms into C++ adapters (or into new Rust modules beyond the
  typed call surface).
- Migrating self-tests wholesale to Qt Test (existing suites migrate call shape only
  where they exercise a landed unit; new GUI checks stay Qt Test only).
- Covering every one of the 254 invokables — only the seven namespaces / five units
  in scope; the rest are named deferrals until later issues claim them.
- Parallel refactor issues #269–#280 (command wiring, panel lifecycle, etc.); they
  route through these adapters when they land, separately.

## Decisions

### D1 — Spike the `Vec<LayerRow>` return shape before unit D (gating)

Two vehicles, same C++ call site either way:

1. `#[qinvokable] fn layers_snapshot(...) -> Vec<LayerRow>` with `LayerRow` as a cxx
   shared struct in the bridge.
2. Free `extern "Rust" fn layers_snapshot(view, ...) -> Vec<LayerRow>` (matches
   `layer_style` pattern; plain cxx codegen, documented `Vec<Shared>` support).

**Outcome (spike landed):** cxx-qt 0.10 codegen accepts (1). The main bridge
carries `#[namespace = "pictura"] struct LayerRow` and
`#[qinvokable] layers_snapshot(thumb_size, entire_document) -> Vec<LayerRow>`;
`cargo check`, `cmake --build build`, both self-test runs, and the app Rust
tests pass on that path. Vehicle (1) is chosen; the free-fn fallback was not
needed.

Side effects of the spike, owned by later units:

- The panel-local `struct LayerRow` in `layers_panel_model.h` was renamed
  `LayerRowNode` so the bridge type can take the name; unit D replaces the
  panel's per-field refresh with the snapshot and can collapse the two.
- The spike body fills path/depth/name from the existing per-field getters;
  unit D widens the struct to every field and fills it in one pass.
- `cxxqt_object.rs` grew 1220 → 1233 LOC, over its allowlisted ceiling of
  1220. The recorded ceiling is not raised; unit A's deletions must bring the
  file back under 1220 before that unit completes (its done criteria include
  `scripts/check-file-size.sh`).

**Alternatives:** packed `QStringList`/`QVariantList` (proven but re-creates a
string protocol); return `QImage` only and keep field getters (keeps N+1).

### D2 — Adapters are C++ façade namespaces, not a second bridge

Each unit is `crates/pictura-app/cpp/api/<unit>_api.{h,cpp}`: free functions and small
value types in `pictura::` (or `pictura::<unit>::`), taking `PictureView&` only inside
the `.cpp`. The header is what consumers include; it may depend on cxx-qt types for
parameters but exposes typed structs for geometry and snapshot rows.

**Why not more Rust bridges?** The Rust surface already grew to ~46 free-fn bridge
modules without a policy; adding adapters in C++ stops *consumer* sprawl without
another codegen layer. The hybrid typed calls (snapshot, geometry) are the only Rust
growth, and they replace deleted invokables (net allowlist shrink).

**What counts as absorbing complexity (vs pass-through ban):** batched reads
(snapshot), typed protocol conversion (string → struct once), ordering guarantees
(Stroke), or a unit façade that centralizes multi-call sequences (document open/save/
dirty + history navigation as one include surface). A 1:1 rename of one invokable does
not count.

**Alternative rejected:** one mega `api.h` umbrella from day one — worse incremental
build and blurs unit ownership; add an umbrella later if include pain shows up.

### D3 — Typed geometry converts once at the adapter

Rust may still format or store geometry as today until a unit flips it; the adapter's
job for string-era units is: one parse (or one typed call) → typed struct → consumers.
When the Rust side returns typed values, the adapter becomes a thin forwarder for that
type only (still not a pure pass-through for the unit: it owns the unit's façade and
all of that unit's string conversions).

`Quad` is 4 points; preview matrix is 9 `f64`; `SelectionContour` is the existing
path list structure typed, not a re-parse of `x,y;…` at every call site;
`SelectionBounds` and `CanvasSize` follow the same pattern.

**Unit ownership for the full inventory:** document → `display_level_size`/`CanvasSize`;
selection → `selection_contour`, `selection_bounds`; transform/paint → `layer_rect`,
`transform_quad`, `transform_preview_matrix`, `image_view` projective/contour parsers;
transform also claims `composite_bounds` for tool_crop/tool_perspectivecrop (or names
a deferral if left for a crop-specific follow-up).

### D4 — Paint stroke is a tool-lifetime object in `paint_api`

Stack RAII alone cannot cover press→release tools (begin on press, end on release in
a different method). The adapter owns a stroke handle owned by the tool:

```cpp
// sketch — paint_api.h
class Stroke {
public:
  Stroke(PaintApi&, const StrokeSpec&);  // begin_paint / begin_brush
  ~Stroke();                             // finish once if still open
  bool dab(...);                         // paint_dab + flush policy
  void present();                        // queues flush_present, never image()
  void finish();                         // end_paint on release
  void cancel();                         // Esc: end without committing
  Stroke(const Stroke&) = delete;
};
```

- Tool holds `std::optional<Stroke>` (or unique_ptr): construct on press, `finish()`/
  `cancel()` on release/Esc; destructor still closes exactly once.
- Migrate the `begin_brush` helper layer into `paint_api` and delete the old helper
  once callers move.
- `frame.cpp:448`'s `QTimer::singleShot(… flush_present)` is migrated to
  `paint_api::presentDeferred()` (or equivalent) in unit C — it is a raw bridge call
  outside tools today.
- Mid-stroke full-frame reads do not exist on `Stroke`.

**Alternative rejected:** assert-only wrapper (weaker than a lifetime object; still
allows skipped `end_paint` on early return without a scope/tool-held guard).

### D5 — One change, five units, one PR

OpenSpec: single change `typed-cpp-api-adapters` (this artifact set), tasks grouped by
unit. User accepts multiple changes if required; one change matches one PR and one
archive. Docs stay on #281 with `TASK-ALLOWS-DOCS`, committed separately.

### D6 — Consumer migration is include-by-include, not a big bang

Within a unit: land adapter + typed/snapshot API + tests → enumerate that unit's
consumer TUs with `rg -l` for its symbols → switch them → delete superseded invokables
when the last caller is gone (grep gate in the unit's task). Self-tests that assert
string equality migrate in the same unit as the deletion (36 assertion hits for the
five named protocols, plus `selection_bounds`/`composite_bounds` assertions —
inventory per unit in tasks). Units without deletable surface (A, E) achieve
exclusivity by consumer migration + include gates only; permanent dual surface is
accepted for those units and does not require a fake deletion.

## Risks / Trade-offs

- **[cxx-qt rejects `Vec<Shared>` on `qinvokable`]** → free-fn bridge (D1); same
  snapshot requirement, no spec change.
- **[Dual surface during each unit]** (adapter lands before string invokable dies) →
  time-box: delete within the unit once callers move; grep task prevents a permanent
  dual surface for units that have deletable surface.
- **[One large PR review load]** → units are independently buildable commits in fixed
  order; reviewers can review per unit; CI runs the full bar per push.
- **[Migration unit misses a consumer]** → compile break (missing header) or grep gate
  in the unit task; raw `PictureView*` outside `cpp/api/` is the smell to re-grep;
  each unit's task starts with an `rg -l` consumer inventory.
- **[tst_layers_snapshot overfits fixture]** → fixture includes styled, vector-masked,
  smart-object, clipped, shape, nested, and adjustment rows so field equivalence is
  real.
- **[Allowlist does not shrink if deletions lag]** → each unit's done criteria include
  the file-size check; final integration task lowers/removes the `cxxqt_object.rs`
  entry when under cap.
- **[Scope creep into pass-through adapters]** → first ADDED requirement rejects 1:1
  forwarders at review; unit façades (document/history/type) must name the multi-call
  sequence they centralize.
- **[Stroke lifetime vs multi-event tools]** → D4 tool-held handle, not stack-only
  RAII; `begin_brush` and `frame.cpp` timer are in unit C's surface list.
- **[frame_includes.h mega-include]** → assign cxxqt lines to owning units (A: new_document;
  D: clipping/layer_masks; export may be a named deferral if no unit owns export);
  task 7.3 gate lists leftovers as deferrals matching the requirement's clause.

## Migration Plan

1. **#281 docs first** (separate commit): `refactor-view-seam.md`, Track 4 supersede,
   glossary terms; `guard.sh` with `TASK-ALLOWS-DOCS`.
2. **Spike commit**: D1 result recorded here; branch build green.
3. **Unit A document+history** → **B selection** → **C paint+transform** (Stroke,
   typed geometry, begin_brush + frame flush) → **D layers+snapshot**
   (`tst_layers_snapshot`, delete 29 `layer_row_*` + style probe loop) → **E type**.
4. Each unit: adapter + consumer inventory (`rg`) + consumers + deletions +
   unit-local tests; push runs full verification.
5. **Final integration**: allowlist shrink if applicable; full bar including fmt,
   clippy, selftest budget; PR
   `refactor: add typed C++ API adapters over the PictureView bridge (#276)`.

**Rollback:** units are additive-then-delete; revert the unit commit pair (add+delete)
if a consumer path was missed. No file format or user-visible behavior changes.

## Open Questions

- None blocking. D1's spike outcome only chooses the vehicle for `layers_snapshot`;
  the spec requires one crossing batch and field equivalence either way.
- Whether `composite_bounds` is deleted in unit C or deferred to a crop follow-up is
  decided at unit C start from the consumer inventory; if deferred, name it in tasks
  7.3 deferrals.