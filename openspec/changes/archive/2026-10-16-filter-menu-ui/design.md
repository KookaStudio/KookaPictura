# Design

## Context

See `proposal.md` — Why. Current state the approach must work within:

- `apply_filter(kind)` (`crates/pictura-app/src/cxxqt_object/impl_filters.rs`) maps a
  kind to a `Filter` with fixed defaults via `filter_from_kind` and mutates the
  active layer through `pictura_render::apply_filter`. There is no way to pass
  runtime values and no path that renders without committing.
- `pictura_render::apply_filter` (`crates/pictura-render/src/filter.rs`) operates
  in place on the layer's colour channels, gated by an optional selection mask;
  it clones the pre-filter plane only internally, so a preview must supply its
  own rollback.
- The menu is a declarative table: `command_tree.cpp` adds leaves and a
  `CommandRegistry` dispatches by id via `setHandler` / `setEnabledProvider` /
  `setLabelProvider` (`commands.h`). Unimplemented leaves are auto-id'd by
  `idFor(path)` and never get handlers.
- New C++ files must be listed explicitly in `CMakeLists.txt` (no globbing), and
  code files target under 800 LOC with a hard 1200 cap.

## Goals / Non-Goals

**Goals:**
- One source of truth for "which filter, what label, path, kind, controls" that
  drives both menu construction and dispatch.
- A runtime parameter path that keeps the existing default behaviour intact for
  the control server.
- A preview that never touches history and always recomputes from pre-filter
  pixels.
- Last-filter state that survives between dialog invocations.

**Non-Goals:**
- The Filter Gallery stack, Smart Filters, Fade, and the disabled stubs listed
  in the proposal.
- CS6 frameless dialog chrome — deferred to issue #85.
- Any change to the filter kernels themselves.

## Decisions

**1. A curated `filter_commands` table supplies the implemented rows; the CS6 tree stays hand-written.**
`command_tree.cpp` keeps the documented CS6 `Filter` tree (families, order, and
the disabled stubs) hand-written as before. `filter_commands.{h,cpp}` holds one
row per implemented leaf: stable id, label, menu path, kind, and the ordered
parameter descriptors (label, min, max, default, decimals, suffix, control
type). `frame_menus_filter.cpp` walks those rows, marks each matching menu id
implemented, and registers the handler and enablement from the same row; a leaf
with no row (Filter Gallery, Reduce Noise, `Custom`, …) stays an unhandled stub.
The Rust bridge recognises a kind through the mapping's
`filter_param_arity`/`FILTER_ARITIES` guard, so table rows and the mapping cannot
drift apart unnoticed. Alternative considered: add ~80 frozen `command_ids`
constants and a separate handler switch — rejected because it splits the
kind/label/control facts into two files that must stay synchronised by hand.

**2. Slot-list parameters across the bridge, defaults preserved.**
Add `filter_from_kind_params(kind, &[f64]) -> Option<Filter>` beside
`filter_from_kind`; each kind has a fixed slot order matching its control order
in the table, and an empty slice produces the documented defaults. `apply_filter(kind)`
becomes `filter_from_kind_params(kind, &[])`. Alternative considered: have Rust
own a parameter schema (label/min/max) serialised to C++ — rejected as more FFI
surface for a port where photorust already fixes the slot order; the order
duplication is contained by a round-trip test over every kind and its arity.

**3. Preview filters a scratch copy and commits on OK.**
When a dialog opens, the bridge snapshots the active layer (or its clamped
rect); each parameter change clones the snapshot, runs `apply_filter` on the
clone, swaps the result into the document, and refreshes the region without
`record`. OK keeps the current result and records one `"Filter"` state; Cancel
restores the snapshot bit-identically and records nothing. This is what makes
"preview from pre-filter pixels" true without an undo-stack round trip.
Alternative considered: apply and undo per change — rejected because it pollutes
history and cannot guarantee the Cancel restore.

**4. The dialog matches the CS6 layout and owns its zoom.**
`FilterPreviewDialog` lays out a thumbnail top-left, OK / Cancel / Preview
stacked on the right, a zoom row (theme magnifier icons with a text fallback,
and a percentage label) beneath the thumbnail, and each parameter's value box on
the label line with its slider below. Radial Blur drops the thumbnail for a Blur
Center; Lens Flare uses the placement pad. Standard `QDialog` chrome (the
frameless treatment stays issue #85). Alternative considered: a bespoke minimal
dialog per filter family — rejected as more code and worse parity.

**5. Last-filter state lives on the bridge.**
`PictureViewRust` stores `(kind, params)` for the last committed filter. The
menu label provider and the two Last Filter commands read it, and the dialog
prefill uses it. Alternative considered: keep it in the C++ main window —
rejected because the commit happens on the Rust side and would need mirroring.

**6. Grayscale layers are filtered as three identical planes.**
`pictura_render::apply_filter` builds three colour planes, but a Grayscale layer
carries only channel `0`; the old code returned `InvalidParams` and the bridge
swallowed it as a silent no-op. Rather than disable filters or convert modes,
the renderer copies channel `0` into all three working planes, runs the filter,
and writes the filtered plane back to channel `0`; a missing channel `0` or a
mixed `1`/`2` layout stays an error. This preserves CS6 parity (filters work in
Grayscale) and is covered by Rust tests plus a Qt test that commits through the
dialog on a Grayscale document.

**7. Dialogs are shown non-modally with a parent input blocker.**
The window compositor dims a modal dialog's parent (KWin's "Dialog Parent"
effect). `runDialog` (`dialogs.{h,cpp}`) shows the dialog with `Qt::NonModal`,
installs an event filter that swallows input addressed to the parent top-level
window, and runs a local event loop until the dialog finishes; the caller gets
the same result a modal `exec()` would. This generalises the existing
file-dialog workaround to every app dialog. Dialogs with no parent keep a plain
`exec()`, and a file dialog that hands off to a platform/portal chooser keeps
the platform's modality.

**8. Transparency is filtered, and previews are viewport-bounded.**
Every filter kernel clamps to three planes, so an unlocked layer's transparency
would otherwise keep a hard edge. Rather than rework ~60 kernels, `apply_filter`
runs the same kernel over a grey copy of the alpha plane and writes the masked
result back (a transparency lock keeps alpha and skips clear pixels). For the
live preview, `apply_filter_region` filters a document rect clamped to the
layer; the dialog passes the visible viewport rect expanded by
`preview_apron(filter)`, so a drag costs at most a viewport while the visible
area matches a full apply. OK still filters the whole layer. The dialog
thumbnail crops that same visible section at the canvas zoom instead of scaling
the whole image down.

## Risks / Trade-offs

- [~2,600 LOC of ported dialog/mapping] → split the dialog into multiple TUs and
  keep descriptors in `filter_commands.cpp`, each under the 1200 cap.
- [Slot order duplicated between the C++ table and `filter_from_kind_params`] →
  the guard is now bidirectional: `filter_param_arity` exposes the mapping's
  slot count per kind, and an all-rows Rust test asserts every kind defaults from
  an empty slice and accepts exactly its arity; the `tst_filter_menu` all-rows
  test asserts every table row resolves to an implemented menu id, so a row or
  dialog whose control count disagrees with the mapping fails.
- [Preview on large documents is expensive] → preview filters the active layer's
  clamped rect and refreshes only that region; a downscaled proxy is a deferred
  performance follow-up, not a behaviour change.
- [The existing `filter-application` spec describes a 3-argument
  `apply_filter` while the code takes a `gpu_enabled` flag] → out of scope here;
  recorded as a pre-existing drift, not silently corrected.
- [Cancel restore correctness] → the pre-filter snapshot is taken before the
  first preview and restored verbatim; covered by a Qt Test.

## Migration Plan

No data or on-disk format change. Rollback is a revert; the control-server
`apply_filter(kind)` path keeps its current defaults throughout.

## Open Questions

- Should preview downscale to a proxy above a document-size threshold? Deferrable:
  it affects only preview latency, not the specs or task breakdown.
