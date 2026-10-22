# Design

## Context

See `proposal.md` — Why — for the motivation. The current state that shapes the approach:

- Four tables hold a kind's parameter facts, and they are kept in step by hand: the C++ row table (`cpp/filter_commands.cpp`, 110 menu rows carrying 494 parameter controls and 615 slots), the `FILTER_ARITIES` table (`filter_map/arities.rs`, 111 rows plus the Lens Blur commit normalization), the Rust mapping's 111 arms (104 repeating a fixed arity inline; together they carry over 300 fallback defaults), and the kind-keyed lists for the positional kinds (`filter_tools.rs`, 15 kinds) and the dedicated-dialog kinds (`frame_menus_filter.cpp`, 11 kinds).
- A mechanical audit of the two big tables (the C++ rows parsed into ordered slot defaults, diffed against the Rust arms slot by slot) established three facts the design leans on: the kind sets agree on all 111 kinds; a kind's canonical arity equals the sum of its row's control slot counts, on every kind; and the two sides' defaults agree on 110 of the 111 kinds. The one disagreement, `lighting-effects`, is in per-slot fallbacks that are unreachable — the mapping only runs with a full `9 + 13k` slot list (the rig plus one light per 13-slot run), so every slot is present.
- The app is a Rust `staticlib` linked into the C++ binary; `cxx-qt` carries Rust values into C++ through shared structs and free functions. `layers_snapshot() -> Vec<LayerRow>` is the existing precedent for a Rust-owned table reaching C++.
- The labels are not translated: there is no `.ts` catalogue in the repository, and the labels are used raw (`new QLabel(spec.label)`, `QCheckBox(spec.label)`), never through `tr()`. Moving them into Rust therefore costs nothing today.
- The engine owns its own defaults for a few kinds (`Lighting::default()`, `FrameOptions::default()`, `TonalFade::default()`), and those are the engine's contract, used by the engine itself.
- House rules that shape the layout: a file targets 800 lines and must stay under 1200 (1400 for tests); a new `.cpp`/`.h` needs an explicit `CMakeLists.txt` entry; `docs/` is read-only unless the task says otherwise and any `docs/` change carries `TASK-ALLOWS-DOCS`.

## Goals / Non-Goals

**Goals:**

- One place records a kind's parameter facts, and both the dialog and the mapping read it.
- A kind's canonical arity is derived, not tabulated, so it cannot disagree with the slots.
- Adding a filter touches one table rather than four.
- The change is behaviour-preserving: the same dialogs open on the same values, the same kinds map to the same filters, and the same refusals happen.

**Non-Goals:**

- Making the engine's `Filter` enum exhaustive or per-variant-property driven — that is #826, and it is independent of this change.
- Translating the filter labels. The project has no catalogue today, and adding one is a separate decision.
- Changing any filter's behaviour, default, range, choice list or slot order.
- Moving the choice *labels* into the engine's enums. The engine's enums carry no display names, and two kinds display the same enum with different words (Smart Blur shows `Low`/`Medium`/`High` over `Quality::{Draft, Good, Best}`), so the words are the descriptor's to hold.

## Decisions

### The descriptor lives in the app crate, in Rust

Rust owns the table; C++ renders its rows from it.

The alternative — keeping the table in C++ and having Rust ask C++ for it — puts the engine-facing defaults on the wrong side of the bridge: the mapping needs them, and C++ cannot serve Rust. The other alternative, a Qt-free crate such as `pictura-filters`, was rejected because the table carries presentation data (labels, suffixes, decimals, menu placement) that the engine must not know about, and because the engine's own defaults are a separate, legitimate concern.

### The descriptor is a flat table, not a builder

```rust
pub enum DialogKind {
    Shared,
    LensFlare,
    LightingEffects,
    PictureFrame,
    ReduceNoise,
    BlurGallery,
    Liquify,
}

pub struct FilterParam {
    pub control: FilterControl,
    pub label: &'static str,
    pub minimum: f64,
    pub maximum: f64,
    pub value: f64,               // the single-slot default
    pub decimals: i32,
    pub suffix: &'static str,
    pub choices: &'static [&'static str],
    pub initial: &'static [f64],  // the multi-slot default
}

pub struct FilterDescriptor {
    pub kind: &'static str,
    pub path: &'static [&'static str], // {"Filter", [family], label}; blur-gallery is {"Filter", "Blur Gallery"}
    pub menu: bool,                    // false for a kind with no menu row
    pub label: &'static str,
    pub params: &'static [FilterParam],
    pub whole_layer: bool,
    pub dialog: DialogKind,
    pub map_after_ok: bool,       // Displace's displacement-map chooser
    pub preview_pane: bool,
    pub stacked: bool,
    pub preview_below: bool,
    pub live_preview: bool,
}
```

A table is data: it diffs cleanly against the table it replaces, it can be asserted against, and a reviewer can read it. A builder DSL would be code, and code cannot be compared field-by-field with what it replaces.

The `FilterControl` discriminant mirrors the C++ enum one-for-one so the C++ adapter is a straight copy of each field. `DialogKind`'s discriminants mirror the C++ `FilterDialogKind` in the same way. `choiceValues` is not in the descriptor because every C++ row fills it with the identity range `0..n-1` and nothing reads it; the adapter can fill it the same way.

### A kind's arity is the sum of its control slot counts

The control-to-slot-count map (`Color` 3, `Placement` 2, `BlurCenter` 2, `ShearCurve` 17, `Matrix` 25, everything else 1) moves from `filter_param_controls.cpp` into Rust, beside the table. `FilterParam::slots()` exposes it; a kind's canonical arity is `params.iter().map(FilterParam::slots).sum()`.

This is what deletes `FILTER_ARITIES`. The audit confirmed the sum already equals the tabulated arity on all 111 kinds, including the awkward ones — the Blur Gallery's 62, Path Blur's 27, Lighting Effects' 22, Custom's 27 — so the table carried no information the rows did not.

The canonical count is the length the shared dialog emits and the row test checks. It is not the only length the mapping accepts: `lighting-effects` accepts `9 + 13k` for 1..16 lights, and the Blur Gallery kinds accept variable pin/path/spin lists and, for `blur-gallery`, its per-effect counts, motion section and flash duration before the shared 13-slot tail. Those decoders stay in `filter_map.rs` and `filter_map/gallery.rs` and keep only their record strides, section sizes and the shared tail size; the descriptor supplies the canonical totals their dialogs emit (one record where the row has one, zero for the combined kind) and the dialog defaults. The alternative, keeping `FILTER_ARITIES` beside the descriptor, was rejected: it would preserve exactly the redundancy this change exists to remove.

### A kind's defaults are one `f64` per slot

`FilterParam` keeps the C++ split: `value` is the single-slot default (sliders, choices, checks, hidden ids, shape indices), and `initial` is the multi-slot default (`Color` 3, `Placement` 2, `BlurCenter` 2, `Matrix` 25). `FilterDescriptor::defaults()` concatenates each parameter's default run: `initial` when it is non-empty, otherwise `value` repeated `slots` times. `ShearCurve` records the curve widget's opening two end points — point count 2, `(-1, 0)`, `(1, 0)`, then zero pairs — because that is the state the dialog opens on; count 0 and this two-point curve describe the same straight line, so the mapping is unchanged. The fill choice adds the eighteenth slot.

So a flat `Vec<f64>` of the kind's slot count is a complete description of its dialog-opening defaults, and the mapping can read `v[i]` where it used to read a literal. The alternatives were a per-slot default of `Vec<f64>` (a second shape to keep in step, and an allocation per kind) and the engine's `Default` impls (several kinds have no engine default struct — the whole Artistic family — and those impls are the engine's contract, not the dialog's).

`initial` MUST be empty or exactly `slots` long. A test asserts every default run's length is its parameter's slot count.

The engine's defaults stay where they are. `Lighting::default()`, `FrameOptions::default()` and `TonalFade::default()` remain the engine's, and the descriptor's slots for those kinds must produce the same filter; a test builds the filter from the descriptor's default vector and compares it to the engine's `Default`. The comparison is on the built filter, not raw slots, because Picture Frame's first slot is the zero-based picker index (33) while `FrameOptions::default().frame` is the engine's one-based frame number (34). The mapping's empty-slot-list path builds from the descriptor, so the two cannot drift apart at the call site.

### `lighting-effects` takes the engine's values

The Rust mapping's `lighting()` helper falls back to `0.0` for the light's intensity, hotspot, centre, angle, size, width and elevation, and for the rig's height, where the dialog and the engine's `Light::default()` say 35, 69, (.5, .5), 325, .6, .32, 50 and 50. Those fallbacks are unreachable, so nothing user-visible is wrong today.

The descriptor takes the engine's values, and the `0.0` fallbacks go. The engine wins because `Light::default()` is documented as CS6's Default style, and the descriptor is the dialog's opening state, which already showed those values.

### The positional flag, the menu flag and the dialog kind move into the descriptor

`filter_preview_needs_whole_layer`'s 15-kind `matches!` list and `frame_menus_filter.cpp`'s 11-kind chain are kind-keyed lists, which is the class of duplication this change removes. They become descriptor fields (`whole_layer`, `dialog`), and the separate post-OK Displace branch becomes `map_after_ok`.

`hdr-toning` leaves the positional list with them: it is an adjustment, not a filter, so the list's entry was unreachable (`filter_kind_positional` is only ever called with a filter kind). The resulting positional set is the 14 filter kinds.

The C++ side switches on the descriptor's dialog kind rather than comparing `spec.kind` strings, and runs the Displace chooser when `mapAfterOk` is set.

### The `blur-gallery` descriptor stays off the menu

`blur-gallery` names the history state and Last Filter, not a menu row; today it is synthesised inside `filterCommandForKind` and is deliberately absent from `filterCommands()`. It becomes a descriptor with `menu: false` and the family path `{"Filter", "Blur Gallery"}` (the path the synthetic spec uses to word the empty-target message), so the menu builder skips it and `filterCommandForKind` finds it — the same shape as today.

The C++ adapter therefore keeps two views of the bridge rows: `filterCommands()` returns the `menu: true` rows, preserving every existing caller and the row-iterating tests, and `filterCommandForKind` searches all rows, including the no-menu-row descriptor. `filterCommandForPath` keeps searching `filterCommands()`, so the no-menu-row kind has no path lookup.

Making it a menu row was rejected: it would put a 62-slot kind through the row-iterating test cases that today only walk real menu rows.

### The bridge carries two shared structs

```rust
#[namespace = "pictura"]
struct FilterParamRow {
    control: i32, label: QString, minimum: f64, maximum: f64, value: f64,
    slot_count: i32, decimals: i32, suffix: QString, choices: QStringList, initial: QList_f64,
}

#[namespace = "pictura"]
struct FilterDescriptorRow {
    kind: QString, path: QStringList, menu: bool, label: QString,
    params: Vec<FilterParamRow>,
    whole_layer: bool, dialog: i32, map_after_ok: bool,
    preview_pane: bool, stacked: bool, preview_below: bool, live_preview: bool,
}
```

`slots` is carried because C++ `FilterParamControls::values()`, `setValues()` and `scaledValues()` need the stride after `slotCount()` is deleted. `path` and `menu` carry the menu placement the single `family` string could not: a top-level leaf, a family child and the no-menu-row `blur-gallery` are three different cases.

`Vec<SharedStruct>` as a shared-struct field is the one construct that has to be proven against cxx-qt before the rest is committed. The fallback is a flat `Vec<FilterParamRow>` carrying a `row: i32` index, grouped C++-side; it costs one extra field and a grouping loop.

The C++ `FilterCommandSpec` gains a dialog kind and a map-after-OK flag; `FilterParamSpec` gains `slotCount`. The dialog classes are untouched.

### The table is split by contiguous runs

The unsplit table is well over the 1200 hard cap (the C++ table alone is 795 lines, and the Rust form adds field names), and would grow with every filter. It is split into `descriptor/mod.rs` (the types, the lookup, the slot arithmetic and `commit_params`) and five modules, each holding a contiguous run of the C++ table's rows:

- `paint.rs` — the two top-level leaves and Artistic
- `blur.rs` — Blur Gallery and Blur
- `distort.rs` — Brush Strokes and Distort
- `render.rs` — Noise, Pixelate, Render and Sharpen
- `stylize.rs` — Sketch, Stylize, Texture and Other

`descriptor/mod.rs` concatenates the five modules' rows in that order, so the bridge returns the rows in the C++ table's order. The menu's own display order is not this table order: it lives in `command_tree.cpp` and is unchanged. `filter_map.rs` keeps the slot helpers; the match moves to `filter_map/mapping.rs` because the descriptor-default reads push the original file past the cap; `filter_map/gallery.rs` keeps the variable decoders. Both files stay under the cap; if `mapping.rs` later nears it, the family seam is available.

### The Lens Blur commit normalization stays explicit

`filter_from_kind_params("lens-blur", &[])` opens on Faster, matching the dialog, but a commit always uses More Accurate. `filter_map/arities.rs::commit_params` performs that normalization today; it moves beside the descriptor and keeps its name. `helpers::filter_from_kind` builds the descriptor's default vector, passes it through `commit_params`, and maps the result, so a parameterless apply is a commit too. The descriptor records the dialog-opening values, not a second copy of the commit values, and `tst_lens_blur_dialog::fasterStillCommitsMoreAccurate` guards the split.

## Risks / Trade-offs

- **A label attached to the wrong slot.** Slot *counts* cannot disagree once both sides read one table, but the mapping's *interpretation* of slot 0 (is it the radius or the threshold?) is still a per-kind fact. → The existing per-kind tests in `filter_map_tests.rs` pin the interpretation of about twenty kinds; the mapping's defaults come from the descriptor, and a new test asserts the mapping's empty-list result equals the result of mapping the descriptor's default vector for every kind. A mislabelled slot therefore shows up as a failed default comparison, not as a silent divergence.
- **A typo in the moved table is not a compile error.** The table is data, so a mistyped label compiles. → The table is a direct move and is reviewed as a diff against the C++ table it replaces; the C++ row comparison test catches any field the adapter would get wrong.
- **The mapping's diff is large.** Over 300 inline defaults become `v[i]` reads, and the helper signatures take `f64` defaults instead of `u8`/`usize`. → The edit is mechanical and script-assisted; the existing `filter_map_tests.rs` covers the semantics of the kinds it touches.
- **The engine's `Default` impls and the descriptor can still drift.** `Lighting::default()` and `FrameOptions::default()` are a second copy of some defaults. → A test builds the filter from the descriptor's defaults for those kinds and compares it to the engine's `Default`, so the copy is checked rather than assumed.
- **The labels can no longer be translated by Qt.** They are not translated today, but a future catalogue would need a translation step for values that live in Rust. → Recorded here as the known cost of the move; a `QCoreApplication::translate` shim would serve it when the project decides to add a catalogue.
- **`filter_commands.cpp` stays in the `-O1` heavy-unit list** of `meta/build-performance` even though the file becomes a small adapter. → Leaving it is zero churn and keeps that requirement literally true; the requirement is not modified.
- **`mapping.rs` may still approach the cap.** → Split on the family seam if it crosses 1200 lines.
