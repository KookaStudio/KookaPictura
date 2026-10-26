# Design

## Context

The current filter facts are split across four seams:

- `crates/pictura-filters/src/filter/variants.rs:8` generates `FilterKind`, `ALL`, `name`, `engine`, and `Filter::kind` from one macro table.
- `crates/pictura-filters/src/filter/props.rs:10` decides `moves_alpha`, `cpu_outruns_gpu`, and the parameter-dependent `preview_apron`.
- `crates/pictura-app/src/cxxqt_object/filter_map/descriptor/mod.rs:78` owns one `FilterDescriptor` per menu kind (114 rows). Each row carries the menu path, label, control schema, ranges, defaults, dialog and preview policy. `filter_tools.rs:487` mirrors it to C++.
- `crates/pictura-app/src/cxxqt_object/filter_map/mapping.rs:39` maps kind plus slots to a `Filter` (1,141 lines). `gallery.rs` owns the variable-length Blur Gallery grammar. `mapping.rs:81` clamps Smart Blur; `photorust/dispatch.rs:105` and `:119` independently range-check Add Noise and Surface Blur.

The shell surfaces also hold copies: `crates/pictura-app/cpp/filter_gallery_dialog.cpp:97` has `kGallery`, and `crates/pictura-script/tests/catalogue.rs:88` parses the descriptor Rust source because the script crate cannot depend on the app crate.

`crates/pictura-render/src/gpu_filter/plan.rs:72` is a parallel kernel match; `crates/pictura-render/src/filter.rs:189` skips the GPU for `cpu_outruns_gpu` kinds.

The closest existing pattern is `imaging/adjustment-catalogue`: one engine table in `crates/pictura-render/src/adjustment_catalogue.rs:83`, one bridge (`engine_tables/adjustment_catalogue.rs`), and the shell builds every adjustment surface from it.

## Goals / Non-Goals

**Goals:**

- One catalogue row per filter kind owns every fact currently split between the descriptor, `props.rs`, `variants.rs`, the gallery table, and the GPU list.
- Adding a fixed-shape filter is one row, its kernel, and contract tests; no descriptor, mapping, or gallery edit.
- The compiler keeps enforcing that every `FilterKind` decides every engine fact, and every variant is applied by exactly one engine.
- The C++ menu/dialog and gallery surfaces keep their current contents and order.

**Non-Goals:**

- Changing filter math, shader kernels, PSD smart-filter encoding, or dialog layouts.
- Moving the hand-laid C++ dialogs (Lens Flare, Lighting Effects, Blur Gallery, Liquify, AWA, Vanishing Point) into Rust.
- Adding a new bridge, QObject, crate, or external dependency.
- Deriving the `pictura-script` command rows from the catalogue in this change. The static command table stays; only its membership test reads the catalogue.

## Decisions

### D1: The catalogue lives in `pictura-filters`, one spec const per kind

`pictura-filters/src/catalogue/` gains the spec rows and the catalog types. The existing `filter_variants!` table grows one trailing token per row:

```rust
filter_variants! {
    GaussianBlur: Filter::GaussianBlur { .. } => GAUSSIAN_BLUR,
    ...
}
```

The macro still generates `FilterKind`, `ALL`, `name`, `kind()`, and `spec()`. It also generates `Filter::engine()` from the row. Each row names a `FilterSpec` const from `catalogue::specs`; the spec const repeats `kind: FilterKind::GaussianBlur` so a mismatched pairing fails `fc_catalogue_complete`.

Why not a separate lookup table indexed by discriminant: the Filter menu order must stay the former descriptor order, which is not the `Filter` enum order. The macro row order is the menu order, and `spec()` returns the named const directly. Reordering the macro rows reorders `ALL` but does not affect the enum's identity.

Why this preserves `imaging/filter-engine-facts`: a new `Filter` variant without a macro row fails `Filter::kind()`'s exhaustive match; a macro row without a spec const fails name resolution; a spec const with a missing field fails the struct literal. The compiler still rejects an undecided fact.

### D2: Row shape

```rust
pub struct FilterSpec {
    pub kind: FilterKind,
    pub id: &'static str,                  // bridge/menu kind, e.g. "add-noise"
    pub path: &'static [&'static str],     // empty for engine-only rows
    pub menu: bool,
    pub label: &'static str,
    pub params: &'static [FilterParam],
    pub dialog: DialogKind,
    pub map_after_ok: bool,
    pub preview_pane: bool,
    pub stacked: bool,
    pub preview_below: bool,
    pub live_preview: bool,
    pub whole_layer: bool,
    pub alpha: AlphaPolicy,
    pub apron: ApronPolicy,
    pub gpu: GpuPolicy,
    pub gallery: Option<u16>,
    pub build: Build,
}
```

`FilterParam`, `FilterControl`, and `FilterDialog` are the current descriptor types moved verbatim, keeping their `#[repr(i32)]` values so the C++ mirror does not change.

Policies are data, not matches in `props.rs`:

- `AlphaPolicy`: `Never`, `Always`, `SmartBlurNormal`.
- `ApronPolicy`: a small enum covering the current formulas (`Fixed`, `RadiusCeil`, `RadiusTimes3`, `RadiusCeilPlus1`, `IntRadius`, `Distance`, `MaxPinBlur`, `TiltBlur`, `PathReach`, `Spin`, `GalleryMax`, `Amount`, `Offset`).
- `GpuPolicy`: `Unsupported`, `Accelerated`, `CpuPreferred`.

`FilterKind::moves_alpha()` keeps its kind-level conservative answer (Smart Blur counts as moving), while `Filter::moves_alpha()` evaluates `SmartBlurNormal` against the mode. `Filter::preview_apron()` and `cpu_outruns_gpu()` read the row.

### D3: The row constructs the filter; app supplies only resources

`FilterSpec::build` has the shape:

```rust
pub type Build = fn(&BuildCtx<'_>) -> Option<Filter>;
```

The fixed-shape and variable-shape decoders move from `mapping.rs` and `gallery.rs` into `catalogue/build/` (split by family). `pictura-app/src/cxxqt_object/filter_map.rs` becomes:

- `struct AppResources;` implementing `FilterResources` over the existing stores.
- `filter_from_kind_params(kind, params)`, which looks up the row, fills defaults for an empty list, and calls the row's builder.
- the existing `commit_params` / `scale_pixel_params` shell helpers.

`FilterResources` has five lookups: Lens Blur depth map, Displace map, Liquify mesh, Adaptive Wide Angle constraints, and Vanishing Point edit. The trait lives in `pictura-filters` because the constructor needs the values and the stores live in the app. `catalogue::default_filter(kind)` uses a `NullResources` implementation that returns the same neutral values the stores return for id 0; tests and the render crate use it for default instances.

Why a trait rather than keeping the five arms in app: the row would no longer own construction for those kinds, and adding a resource-backed filter would still edit the mapping. Why not move the stores into `pictura-filters`: they are process-global session state with Qt-adjacent lifecycle; the engine crate stays free of that. The trait is one implementation, which the no-abstractions rule normally rejects, but it is the minimal seam that keeps the constructor in the row without importing app state.

### D4: Range source

`FilterSpec::range_for_slot(slot)` walks the parameter list, skipping each control's `slots()`, and returns the containing control's range. `photorust::dispatch::plan` replaces scalar literals with `check_slot(filter, slot, value)`. Mapping clamps that restate a control range are deleted. Structural rules that are not a control range stay explicit: Wave's dependent wavelength/amplitude limits, Shear's per-point `-1..1`, and texture option bounds.

The canonical values come from the docs, not from whichever copy happens to exist:

- Add Noise amount: `0.1..400` (`docs/06-filters/noise-filters.md`).
- Surface Blur threshold: `1..255` (`docs/06-filters/blur-filters.md`).
- Smart Blur radius/threshold: `0.1..100` (already agreed by the descriptor; the engine rejects outside it).

The same audit corrects the remaining dialog drift to the values the engine specs already require: Twirl `-999..999`, Emboss angle `-360..360` and height `1..100`, Patchwork square size `0..10` and relief `0..25`, Photocopy detail `0..24`, Plaster smoothness `0..15`. These are contract fixes, not new engine behavior.

The surface threshold change is a behavior change for programmatic filters that pass 0; no existing test or UI path relies on it.

### D5: GPU policy gates the existing plan

`gpu_filter::plan` starts with `if filter.kind().spec().gpu == Unsupported { return None; }` and otherwise keeps the parameter-dependent match. `filter.rs` keeps using `cpu_outruns_gpu` for the CPU-preferred set.

`gpu_filter_parity::accelerated_filters` gains a cross-check: the set of kinds in the fixture equals the set of catalogue kinds whose policy is `Accelerated` or `CpuPreferred` and whose kind has a GPU kernel. A new accelerated row without a fixture case fails the test; a fixture case without an accelerated row also fails. This closes the "silently stays on CPU" hole without rewriting the parameter match.

Why not a `GpuKernel` enum in the row: it would still need the same parameter match for Gaussian versus Unsharp versus High Pass, and it would add a second enum to keep in step. The policy plus fixture check is the smaller contract that catches the failure mode.

### D6: Gallery rank through the existing bridge

`FilterDescriptorRow` and C++ `FilterCommandSpec` gain `gallery_rank` (`i32`, `-1` for none). `filter_gallery_dialog.cpp` deletes `kGallery`; `categories()` iterates the menu rows from `filterCommands()`, keeps `galleryRank >= 0`, groups by `path.at(1)`, and sorts groups by their lowest rank and rows by rank.

Ranks are assigned in the old `kGallery` order (Artistic 0-14, Brush Strokes 15-22, Distort 23-25, Sketch 26-39, Stylize 40, Texture 41-46). This preserves the category order and inner order asserted by `tst_filter_gallery::categoriesFollowCs6`. A new gallery filter sets one rank; the C++ file is not touched.

### D7: Script membership test reads the catalogue

`pictura-script` gains an internal dev-dependency on `pictura-filters`. `every_wired_filter_has_a_definition` replaces the descriptor-directory scan with `pictura_filters::catalogue::all().filter(|row| row.menu)`. The static `command_catalogue/entries.rs` rows stay; adding a filter still adds one command row. This is the smaller change than deriving a 593-row frozen table and reworking `golden_catalogue_matches_the_captured_table` and `default_catalogue_shape_is_stable`.

### D8: HdrToning is an engine-only row

`HdrToning` has a `FilterKind` but is deliberately absent from the 114 menu rows. It gets a row with `path: &[]`, `menu: false`, `params: &[]`, `gallery: None`, `dialog: Shared`, `whole_layer: false`. Its builder returns `Filter::HdrToning(HdrToningParams::default())` for an empty list and refuses non-empty lists. `filter_kind_supported` requires a non-empty command path, so HDR Toning stays out of the Filter menu, gallery, and scriptable filter command set. `filterCommandForKind` still resolves the no-menu row, as `imaging/filter-app-ui`'s `fpd_cpp_rows` scenario requires, and the app's filter-command entry points reject empty-path rows before applying them.

### D9: File layout

- `pictura-filters/src/catalogue/mod.rs`: types, accessors, policies, defaults/range helpers.
- `catalogue/rows/paint.rs`, `blur.rs`, `distort.rs`, `render.rs`, `stylize.rs`, `other.rs`, `video.rs`, `engine_only.rs`: spec rows split by family.
- `catalogue/build/mod.rs`, `build/blur.rs`, `build/distort.rs`, `build/render.rs`, `build/stylize.rs`, `build/paint.rs`, `build/gallery.rs`: decoders split so no file crosses the code cap.
- `pictura-app/src/cxxqt_object/filter_map.rs`: resource impl and FFI-facing helpers only.
- `filter_map_tests.rs` stays in app (it exercises the wrapper and the app stores) and updates its descriptor paths to the catalogue.

Pure moves are made first, then the behavior changes (range fixes, gallery rank, GPU gate), so a bisect can separate the mechanical relocation from the behavior change.

## Risks / Trade-offs

- **Mechanical move of ~4,000 lines** -> do it in pure-move commits per family, with the existing `filter_map_tests` and catalogue tests as the oracle; no kernel edits.
- **The resource trait has one implementation** -> keep it to five lookups with no default methods; if a second consumer appears, it is already the right seam.
- **Range unification changes programmatic acceptance** -> Add Noise `0.0` and Surface Blur threshold `0` become refusals. Add explicit tests for the boundary values and check that no existing test constructs them.
- **Macro row plus spec const is still two edits per new kind** -> the enum variant, macro row, and spec const are the price of compile-time fact completeness; the descriptor, mapping, gallery, and GPU list edits disappear.
- **Static script rows remain** -> one command row per new filter. Deriving them is a separate change because it changes a frozen 593-row table.
- **Gallery rank is data that must stay ordered** -> the C++ `categoriesFollowCs6` test is the guard; a duplicate or missing rank would collapse or misorder a category.
- **`FilterKind::ALL` order changes to the former menu order** -> no code serializes the discriminant; the existing tests build sets, not sequences. The oracle mapping and digest tests iterate the list and are order-independent.

## Migration Plan

No persisted data or public file format changes. The move lands in one change:

1. Add the catalogue types, rows, policies, and default/range helpers; wire the macro and `props.rs` to them. Keep the existing descriptor and mapping as the behavior oracle.
2. Move the decoders into `catalogue/build/`, add `FilterResources` and `AppResources`, and delete the app descriptor/mapping/gallery modules.
3. Fix the Add Noise and Surface Blur ranges; make dispatch read slot ranges.
4. Add `gallery_rank` to the bridge and C++ spec; delete `kGallery`.
5. Switch the script membership test and GPU parity cross-check to the catalogue.
6. Delete the old descriptor/mapping tests or move their assertions to the catalogue; update the literal `114` to `FilterKind::ALL.len()`.

Rollback is a revert of the change; there is no migration artifact.
