# Design

## Context

See `proposal.md` for motivation and the spec deltas for the required behavior.

The engine already owns the only layer blend-mode table:
`BlendMode::LAYER_MODES` in `crates/pictura-core/src/lib.rs`, published to the
shell as `blend_mode_rows()` and cached in C++ as `blendModes()`. The missing
piece is availability. The document state that decides it already exists:

- `pictura_render::document_color_mode(doc)` returns the recorded source mode
  (Lab, CMYK, Indexed, …) or the working mode. It is the value `channels_mode`
  shows in the Channels panel.
- `pictura_render::document_bit_depth(doc)` returns `One` for Bitmap, the
  retained native depth for a 16/32-bit source, else `Eight`.

Fade already gates its own menu through `fade/imp.rs::mode_allowed`, but with a
hand-written six-mode Lab list that disagrees with the documented eight. The
other controls do no gating.

Two implementation constraints shape the approach:

- `crates/pictura-core/src/lib.rs` is at 1190 of its 1200-line cap, so a
  predicate cannot grow it.
- `crates/pictura-app/cpp/panels/layers_panel.cpp` is at 1260 lines with a
  1262-line allowlist ceiling, so the panel has two lines of headroom; the
  planned change uses one (1261).

## Goals / Non-Goals

**Goals:**

- One availability table, owned by the engine, used by every layer blend-mode
  authoring control.
- Gate on the document the user sees, not the RGB working planes.
- Keep the engine list order and the Layer Style stored engine-index contract
  intact while filtering.
- Keep the unavailable stored mode visible and truthful.

**Non-Goals:**

- Refusing to open, save, or convert a document that stores an unavailable
  mode; that policy belongs to the global gating work (#490).
- Filtering the Layers panel's Mode filter list (it is a search query) or the
  Apply Image blending list (its own documented list and `Add`/`Subtract`
  naming).
- Adding `Pass Through` to group combos.
- Changing blend arithmetic; the compositor keeps blending all modes.

## Decisions

### Put the predicate in `pictura-core/src/blend.rs`

Add an inherent `BlendMode::allowed(self, ColorMode, BitDepth) -> bool` next to
the blend functions, not in `lib.rs` (line cap) and not as a new module. The
restriction is conjunctive:

```
Bitmap | Indexed -> self == Normal
otherwise        -> (mode != Lab || self not in {ColorDodge, ColorBurn, Darken,
                                                   Lighten, Difference, Exclusion,
                                                   Subtract, Divide})
                    && (depth != ThirtyTwo || self in the 15-mode HDR list)
```

Both restrictions are evaluated, so a malformed document that presents Lab at
32-bit yields the intersection of the two sets, never the more permissive
Lab-only set. `PassThrough` is never passed by the layer list and is not
special-cased.

*Alternative considered:* the docs' proposed `pictura_blend::restrict` module.
There is no `pictura-blend` crate; a module for one predicate would be a new
seam for no consumer. A method on the enum is the smallest home with the same
single-table property.

### Publish the allowed keys per view from `engine_tables`

Add `allowed_blend_mode_keys(view: &PictureView) -> Vec<QString>` to the
existing `engine_tables` bridge, implemented in
`engine_tables/blend_modes/imp.rs`. It reads the document's effective mode and
depth and filters `LAYER_MODES`; without a document it returns all 27. The pure
mode/depth-to-keys part stays a separate function so it is unit-testable
without a `PictureView`. The bridge must declare the view type in its
`unsafe extern "C++"` block — `include!("pictura_app/src/cxxqt_object.cxxqt.h")`
and `type PictureView = super::PictureView;` — following `channels.rs`.

*Alternative considered:* expose `document_color_mode` and let C++ implement
the table. That would put a second availability table in the shell, which the
existing spec forbids for keys/names and would drift again like Fade did.

### Filter through `blend_modes.cpp`, not the panel

Add to `crates/pictura-app/cpp/blend_modes.{h,cpp}`:

- `QVector<BlendEntry> allowedBlendModes(const PictureView*)` — the engine rows
  whose keys the bridge returns.
- `int syncBlendCombo(QComboBox*, const PictureView*, const QString& currentKey)`
  — computes the desired entries (allowed modes, plus `currentKey` appended when
  it is a layer-mode key that is not allowed), compares them with the combo's
  current items, rebuilds with a `QSignalBlocker` only when they differ,
  disables the appended current entry, and returns `findData(currentKey)`. The
  group sentinel `"pass"` is ignored: a group row keeps the existing Normal
  fallback and never gains a disabled `pass` entry
  (`tst_blend_mode_list::layersComboLeavesGroupPassThrough`).

`blend_modes.h`'s "a combo index is the engine index" comment becomes false for
the filtered Layer Style combo and is reworded.

The comparison is stateless, so no cached gate belongs to the dialog or panel.
A refresh costs one small bridge call and up to 27 comparisons. The Layers
panel replaces its two-line `findData`/`setCurrentIndex` block with the
`syncBlendCombo` call and its index apply, and runs the call on every
`syncControls` — with an empty current key when no row is selected — so the
list is never left stale. The panel ends at 1261 lines, one under its 1262
allowlist entry. All rebuild logic stays in `blend_modes.cpp` (29 lines today).

*Alternative considered:* disable items in place and keep all 27. The contract
says the layer menu hides the unavailable modes; hiding also makes keyboard
cycling land only on selectable modes. The conflict entry is the one exception,
because hiding it would misrepresent the layer.

### New Layer dialog takes an optional view

`LayerNewDialog` gains an appended defaulted `PictureView* view = nullptr` on
both constructors (`LayerNewDialog(bool group, QWidget* parent = nullptr,
PictureView* view = nullptr)`) and on `get` (`get(bool group, QWidget* parent,
LayerNewSpec* out, const QString& defaultName = {}, PictureView* view =
nullptr)`), with `nullptr` meaning "no document context, show all 27". Existing
direct construction (`selftest_layers_controls.cpp`, `tst_value_input_enter.cpp`,
`tst_blend_mode_list.cpp`) and the name-and-color variant stay unchanged. The
five `get` sites are 3 in `layers_panel_actions.cpp`, 1 in
`fill_layer_editor.cpp`, and 1 in `tst_fill_layer_editor.cpp`; the four
production sites pass their current view and the test keeps the null default.
`syncNeutralForMode` looks its entry up by key instead of assuming the combo
index is the engine index.

*Alternative considered:* pass a filtered entry list from each caller. More
plumbing at every call site for the same result.

### Layer Style combos keep engine indices in item data

`bindBlendMode` currently adds all 27 names and uses the combo index as the
stored value. The rewritten helper adds only allowed entries but stores the
engine index in `itemData`; the changed-signal writes `itemData` back. If the
stored index (or an absent effect's default) is unavailable, it is appended as
a disabled entry so the dialog shows the real mode. It keeps `bindChoice`'s
`std::isfinite` guard: a group or background has no `blending.mode` value
(PassThrough has no layer-mode index,
so the value is NaN), so the combo stays at its default and nothing is
appended; `tst_layer_style::groupBlendingOptions` asserts that. This preserves
the stored-value contract for `blending.mode` and all ten effect modes without
touching the Rust style model. An 8-bit RGB document still produces an
unfiltered list where display index equals engine index; the existing
`layerStyleComboIsEngineIndexed` test is updated to assert the engine index in
`itemData`.

*Alternative considered:* store PSD keys for style modes. That changes the
serialized/typed style model and existing files; too invasive for a menu gate.

### Fade uses the shared predicate

`fade/imp.rs::mode_allowed` becomes a thin call to `BlendMode::allowed` with the
document's effective mode and depth. The dialog keeps its current
show-but-disabled presentation, so the change is behavioural only: Lab gains
`Subtract` and `Divide` as disabled, and 32-bit documents disable the 12 modes
outside the HDR list. This is a requirement change and is captured in the
`imaging/filter-fade` delta.

*Alternative considered:* keep the private list and add the missing entries.
That is how the drift started; the shared predicate is the same amount of code.

### Correct the 32-bit counts in the docs

`docs/05-layers/blend-modes.md` prints 15 mode names but calls them 14 and says
13 are hidden (five occurrences). The same 15 names appear in
`docs/04-image-ops/32-bit-hdr.md` and `bit-depth-and-conversion.md`, and
`layer-groups.md` says 15. The implementation follows the names, and a separate
docs commit corrects the counts and updates the 32-bit/Lab "refuse or convert,
never silently blend" edge case to the interim disabled-current-mode policy
pending #490, with `TASK-ALLOWS-DOCS`.

## Risks / Trade-offs

- **A stored unavailable mode keeps compositing with its mode.** → The control
  shows it selected and disabled, so the state is visible; clamping or refusing
  it is deferred to the global gating policy (#490), and the docs edge case
  records that interim policy.
- **Rebuilding a combo while it has focus or an open popup.** → Only a gate
  change (document switch, mode/depth conversion, or a different unavailable
  current mode) triggers a rebuild, and a filter never changes during a popup
  (popups block document edits). The rebuild is signal-blocked and the panel
  re-selects after it.
- **Qt still allows keyboard navigation onto disabled model items in some
  styles.** → The Fade dialog already uses the same disabled-item pattern; the
  appended conflict entry is only reachable as the current value and is never
  written back unless the user changes it.
- **FFI cost on every layer-panel refresh.** → One call returning at most 27
  short keys; the panel already makes several bridge calls per refresh.
- **Effect-mode style values are indices, not keys.** → `itemData` preserves the
  engine index and the test asserts a chosen filtered entry writes the correct
  mode.
- **Malformed documents with Lab at 32-bit.** → The restriction sets intersect;
  the result is a subset of both, never more permissive.
- **Bitmap and Indexed reachability.** → A Bitmap document has no layers, but
  `convert_to_indexed` flattens to one layer and the add-layer bridge has no mode
  guard, so an Indexed document can present a populated layer combo. The
  Normal-only rule is enforced in the predicate and covered by
  `tst_blend_mode_list::layersComboFollowsIndexed` as well as the unit test.

## Migration Plan

No data migration. PSDs round-trip unchanged; the gate only affects which values
an authoring control offers. Reverting the change restores the previous menus
without touching document data.
