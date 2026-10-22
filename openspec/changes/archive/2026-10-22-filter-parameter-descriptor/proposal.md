# Proposal

## Why

A filter's parameter facts — its slot order, arity, control kinds, labels, ranges, choices, default values, whether it is positional, and which dialog edits it — are spread across four hand-synced tables in two languages, kept in step by hand:

- `crates/pictura-app/cpp/filter_commands.cpp` — 110 menu rows carrying 494 parameter controls (615 slots), plus menu placement and dialog flags. The table alone is 795 of the file's 1055 lines.
- `crates/pictura-app/src/cxxqt_object/filter_map/arities.rs` — a 111-row `FILTER_ARITIES` table and the Lens Blur commit normalization.
- `crates/pictura-app/src/cxxqt_object/filter_map.rs` — 111 mapping arms; 104 repeat a fixed arity inline via the `arity!` macro and the rest decode variable lists, and together they carry over 300 fallback defaults.
- `crates/pictura-app/src/cxxqt_object/filter_tools.rs` — a 15-kind `matches!` list of the positional kinds, plus `cpp/frame_menus_filter.cpp`'s 11-kind chain of the kinds that own a dedicated dialog.

The Rust docstring at `filter_map.rs:6-8` says a kind's slot order "must stay in lock-step with `cpp/filter_commands.cpp`", and the table's own docstring at `filter_map/arities.rs:1-3` says it "keeps the Rust mapping and `cpp/filter_commands.cpp` in lock-step". The cross-language guard is `tst_filter_menu::everyRowsKindIsSupportedAndArityMatches`, which checks the kind is supported, the dialog's value count equals the arity, and a preview opens and cancels — but it does not compare labels, ranges or choices. Per-kind tests pin individual dialog defaults and ranges, but no test compares the two tables field by field, so a changed label, range, choice or default can drift silently. Recent filter feature commits touched 17 to 49 files each (Spin Blur 48, Path Blur 49, Iris Blur 33, Picture Frame 35).

A mechanical audit of the two tables (the C++ rows parsed into ordered slot defaults, diffed against the Rust arms slot by slot) found the kind sets and the arities already agree on all 111 kinds, and the defaults agree on 110 of them. The duplication is therefore not a live bug today — it is a standing manual obligation that every future filter inherits.

## What Changes

- Rust becomes the single source of the filter parameter descriptor. A new `filter_map/descriptor/` module holds one table of descriptors — kind, menu path and whether the kind has a menu row, label, the ordered parameter controls (control kind, label, range, decimals, suffix, choices, default), the whole-layer flag, the dialog kind and the post-OK displacement-map flag.
- The C++ `FilterCommandSpec` rows are built from the Rust descriptors through the cxx bridge, and `cpp/filter_commands.cpp` becomes a thin adapter instead of a table.
- The hand-written `FILTER_ARITIES` table is deleted. A kind's canonical arity is the sum of its descriptor's control slot counts, and the mapping's `arity!` checks read that count instead of repeating a literal. The mapping still accepts the variable lists its requirements document (`lighting-effects`' 1-to-16-light rig and the Blur Gallery record lists); the descriptor records their canonical counts.
- The Rust mapping reads its defaults from the same descriptor, so a kind's opening dialog values and the values its empty slot list maps to are one fact, not two. The one exception stays explicit: Lens Blur's Preview radio is preview-only, so a commit always uses More Accurate.
- The positional-kind list moves into the descriptor, and the dedicated-dialog chain and the post-OK Displace flag become descriptor fields, so no separate kind-keyed lookup list survives outside the descriptor and the mapping's own match.
- The dead entries go: `hdr-toning` (an adjustment, not a filter) leaves the positional list, and the stale `filter_map.rs` comment claiming Custom "is left unmapped" is corrected — `custom` has been mapped since the Filter menu landed.
- Two stale claims in the capability are corrected. Radial Blur's Blur Center is an engine slot pair, not a display-only control, and `Custom`/`Reduce Noise` are implemented kinds, not disabled stubs.
- No user-visible behaviour changes. The one value that moves is the unreachable per-slot fallback of `lighting-effects`, which takes the engine's documented `Light::default()` values.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `imaging/filter-app-ui`: the "Filter-kind mapping and defaults" requirement names the `FILTER_ARITIES` table in `filter_map/arities.rs` as the record of the recognised kinds and their slot order; that table is replaced by the filter parameter descriptor, and a new requirement records the descriptor as the single source of a kind's parameter facts. The "Filter parameter dialog" requirement drops the false claim that Radial Blur's centre records no engine slot, and "Filter menu wiring" drops the false claims that Custom and Reduce Noise are disabled stubs.

## Impact

- **Rust app crate.** New `crates/pictura-app/src/cxxqt_object/filter_map/descriptor/` (types, lookup and slot arithmetic in `mod.rs`; the table split into five contiguous runs of the C++ table's rows: top-level leaves plus Artistic, Blur Gallery plus Blur, Brush Strokes plus Distort, Noise plus Pixelate plus Render plus Sharpen, and Sketch plus Stylize plus Texture plus Other). `filter_map.rs` keeps the slot helpers, the mapping match moves to `filter_map/mapping.rs` because the descriptor-default reads push the file past the cap, and `filter_map/gallery.rs` keeps the variable decoders; the `arity!` checks read the descriptor's count; `filter_map/arities.rs` is deleted, its commit-time Lens Blur normalization moving beside the descriptor; `filter_map/gallery.rs` keeps only the record strides, section sizes and the shared tail size for the variable decoders; the canonical totals and defaults come from the descriptor; `filter_tools.rs` loses the positional list; `helpers.rs`'s parameterless mapping builds the descriptor defaults and passes them through the moved commit normalization.
- **C++ app crate.** `cpp/filter_commands.cpp` becomes an adapter; `cpp/filter_param_controls.cpp` loses its `slotCount()` map (Rust owns the control-to-slot count); `cpp/frame_menus_filter.cpp` switches on the descriptor's dialog kind and map-after-OK flag instead of comparing `spec.kind` strings.
- **Bridge.** Two shared cxx structs carry the descriptors to C++. `Vec<SharedStruct>` as a shared-struct field is the one construct that has to be proven against cxx-qt before the rest of the design is committed; the fallback is a flat parameter list with a row index.
- **File size.** The unsplit table is well over the 1200 hard cap (the C++ table alone is 795 lines, and the Rust form adds field names), so it is split into five contiguous runs. `filter_map.rs` keeps the helpers and `filter_map/mapping.rs` holds the match; both stay under the cap.
- **Docs.** `docs/dev/STATE.md` and `docs/dev/doc-code-parity-gaps.md` cite the moved files and line numbers; they are refreshed in a separate commit carrying `TASK-ALLOWS-DOCS`. FILT-3 is removed because commit `98fa943e` fixed it. The behavioural corpus under `docs/06-filters/` is unchanged, because no value changes.
- **Dependencies.** None added.
