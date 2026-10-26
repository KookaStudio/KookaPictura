# Proposal

## Why

Adding or changing one filter still edits roughly ten source files and five or six test files across three crates, because the engine facts, menu data, parameter schema, defaults, ranges, gallery membership, and slot construction are duplicated. The copies have already drifted: Add Noise accepts `0.0..400` in the ported engine but opens at `0.1..400` in the dialog, and Surface Blur rejects a threshold of 1 in the dialog while the documented range and engine accept it. A filter kind missing from the GPU plan silently stays on the CPU, and `FilterKind` completeness is checked only by a literal `114`.

## What Changes

- Move the destructive filter catalogue into `pictura-filters`. One `FilterSpec` row per `FilterKind` owns the engine, alpha policy, preview apron, CPU/GPU policy, whole-layer policy, parameter schema (control, range, default, choices, suffix, decimals, pixel distance), menu path/label, dialog and preview policy, gallery rank, and slot-to-filter constructor.
- Derive `FilterKind::ALL`, `name`, `engine`, and the fact queries from those rows. A kind without a row does not compile, preserving the existing exhaustive-facts guarantee.
- Reduce `pictura-app`'s `filter_map` to the app-only resource seam (Lens Blur depth maps, Displace maps, Liquify meshes, Adaptive Wide Angle constraints, Vanishing Point edits) and the FFI mirror to C++. The fixed-shape and variable-shape slot decoders move to the catalogue.
- Make the row's parameter ranges the only range source. The ported dispatcher validates scalar slots against the row instead of duplicating range literals. Add Noise becomes `0.1..400`; Surface Blur threshold becomes `1..255`. The same audit corrects the Twirl, Emboss, Patchwork, Photocopy, and Plaster dialogs to the ranges their engine specs already require.
- Make `pictura-render`'s GPU filter plan consume the row's eligibility and add a cross-test that every accelerated row has a parity case and every parity case names an accelerated row.
- Carry the gallery rank through the existing `filter_descriptors` bridge and delete the C++ `kGallery` table. The gallery groups by menu family and sorts by rank.
- Make the `pictura-script` catalogue membership test read `pictura_filters` instead of scraping descriptor Rust source. The static command rows remain; adding a filter still adds one command row beside its `FilterSpec` row.
- Represent `HdrToning` as an engine-only row (`menu: false`, empty path and parameter list) so every `FilterKind` has exactly one spec while it stays out of the Filter menu and scriptable filter commands.

## Capabilities

### New Capabilities

- `imaging/filter-catalogue`: the engine-owned filter catalogue and the single-source rules for kind facts, parameter ranges, slot construction, GPU eligibility, menu/gallery membership, and consumer checks.

### Modified Capabilities

- `imaging/noise-filters`: the Add Noise requirement is replaced by *Add Noise with a 0.1 amount floor*; `0.0` is refused and the zero-amount no-op scenario is removed.
- `imaging/filter-app-ui`: the filter parameter descriptor admits engine-only rows with no menu path and no gallery rank.

Filter rendering, menu contents, gallery contents, and script command semantics are otherwise unchanged; the dialog ranges that had drifted from the engine/spec ranges are corrected.

## Impact

- `crates/pictura-filters`: new `catalogue` module with the spec rows, parameter/control types, engine-fact policies, builders, variable-shape decoders, and catalogue tests. `filter/variants.rs` and `filter/props.rs` delegate to the rows. No new external dependency.
- `crates/pictura-app`: `filter_map` keeps only the resource trait implementation and a thin `filter_from_kind_params`; the descriptor table and mapping/gallery decoders are deleted. `filter_tools`, `helpers`, `canvas_proxy`, and the FFI row gain the catalogue accessor and gallery rank. Internal dev-dependency on `pictura-filters` for the script test only.
- `crates/pictura-render`: `gpu_filter::plan` gates on the catalogue's GPU policy; `gpu_filter_parity` cross-checks its accelerated fixture against the catalogue.
- `crates/pictura-app/cpp`: `filter_commands.h/.cpp` mirror the gallery rank; `filter_gallery_dialog.cpp` deletes `kGallery` and builds categories from the bridge.
- `crates/pictura-script`: `tests/catalogue.rs` reads the filter catalogue; the command table and frozen ids are untouched.
- Tests: catalogue completeness, default-in-range, range single-source, build/default round-trip, resource-backed build, GPU parity list, script membership, and the existing Qt menu/gallery suites.
- File sizes require the catalogue rows and builders to split by filter family; no allowlist increase.
