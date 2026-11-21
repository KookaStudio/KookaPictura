# Design

## Context

See `proposal.md` for the motivation and `specs/` for the behavior contract.

The current View menu is generated from the Rust command catalogue
(`crates/pictura-script/src/command_catalogue/entries.rs`) into the C++ menu
bar; 27 View rows still carry `implemented: false`. Handlers live beside the
feature they drive (`frame_commands.cpp`, `frame_guides.cpp`, `frame_proof.cpp`,
`tool_*.cpp`), and session state is the `SessionState` struct loaded and saved
by `session.cpp`.

The overlays already exist in pieces:

- Selection contours, guides, smart guides, annotations (color samplers, notes,
  counts), slices, and the work-path overlay each have a paint path in
  `ImageView`, but only guides and smart guides have persisted view flags.
- Slices and paths are owned by their tools: leaving the Slice/Pen/Path tools
  clears the overlay.
- Guides and smart guides persist through `guidesVisible` and
  `smartGuidesVisible`; the `frame_guides.cpp` toggle/handler pattern applies
  them to every canvas and skips documents still loading.
- `Document::slices` is the user-slice store; auto slices are derived by
  `pictura_render::resolve_slices`. There is no clear operation yet.
- The zoom tool context menu already implements `200%` and `Print Size`
  (`96 / ppi`).
- The Paths panel tracks an active path (Work Path, saved path, or shape layer
  path), exposed by the path bridge.

Constraints: one change per PR, zero open changes, explicit CMake source
fragments, the Qt Test growth path for GUI checks, a test-plan row per scenario,
and the 800/1200 LOC code cap and 1400 LOC test cap.

## Goals / Non-Goals

**Goals:**

- One application-wide View overlay state, persisted and applied to every
  canvas, replacing the per-canvas ad-hoc flags.
- Make `Extras`, `Show`, `All`, `None`, and `Show Extra Options` behave as one
  coherent model, with the membership set bounding only the `Ctrl+H` master.
- Keep overlay ownership understandable: tools keep their editing decorations,
  the shared refresh supplies the base overlay, and one place computes effective
  visibility.
- Reuse existing engines for zoom, paths, slices and annotations rather than
  introducing parallel implementations.

**Non-Goals:**

- Snap targets (#305), 32-bit preview (#570), Gamut Warning, Color Blindness,
  Pixel Aspect Ratio, CC-only additions, and 3D Axis.
- Changing document pixels, history, save output, or PSD resources.
- A general menu-customization or workspace system (#501).
- Reworking the guides/smart-guide geometry or snapping algorithms.

## Decisions

### State lives in `SessionState`, not on each `ImageView`

Add one persisted view-state block: per-item enabled flags, the color-sampler
flag, the `Extras` master, the membership list, the grid
appearance/interval/subdivisions, and `selectionEdges`. `ImageView` receives the
effective values through one
`setViewOverlayVisibility`-style call instead of owning menu-visible state.

Alternatives: keep the current per-canvas `selectionEdgesVisible_` and add more
per-canvas flags (rejected: sibling windows diverge and a new document inherits
nothing); store the flags per document (rejected: Photoshop's View aids are
application view state, not document data, and must not enter history or PSD).

### Visibility is `enabled && (extras || !member)`

`Show` controls whether an extra is enabled. `Extras` hides enabled members
without changing any checkmark. A non-member keeps its own visibility while
Extras is off. Color samplers have their own visibility flag and are always
members; `Show > All` sets it and `Show > None` clears it. `Show > All` enables
every item and turns Extras on; `Show > None` disables every item; neither
consults the membership set. Extras is the final master: an owning tool may
override only the per-item enabled term for its own overlay, never Extras.

Alternatives: membership also restricting All/None (rejected: Adobe describes
All/None as "all available Extras" and describes membership in the context of
the `Ctrl+H` shortcut); `Extras` clearing per-item flags (rejected: Photoshop
keeps the Show checkmarks while Extras hides the aids); letting the owning tool
override Extras (rejected: `Ctrl+H` hides the aids even while the tool is
active).

### Document gating and handler guards

The Show entries, `Extras`, `All`, `None` and the zoom presets are
document-scoped; `Show Extra Options` and `Lock Slices` are application-global.
Enable providers grey the menu, but `CommandRegistry::dispatch` ignores the
enabled state, so every document-scoped handler also checks `has_document()`
before mutating state or opening a dialog. A disabled dispatch therefore leaves
the stored state unchanged, including for programmatic dispatch.

### `Show Extra Options` is a modal membership window

The window lists the implemented Show extras available in the running edition —
nine in Standard, ten in Extended (Count only in Extended) — all checked by
default, and states that the set controls what `Ctrl+H` hides. Color samplers
are excluded, per Adobe's note that they follow Extras/All/None without a Show
entry. 3D Axis is excluded because it has no engine and stays disabled. The set
persists; OK applies, Cancel discards.

Alternatives: a submenu (rejected: Photoshop presents a window); a set of named
groups (rejected: CS6 and Adobe's shortcut description describe one set, not
named groups).

### Selection Edges resets on a replacement-only generation

`PictureView::selection_revision` is too broad: it also bumps on in-place mask
edits, selection moves, history restores and Quick Mask updates. Add a
replacement-only selection generation bumped only by `set_selection` and
`take_selection`. The frame keeps the app-wide Selection Edges flag, stores the
last-seen generation per document view, and re-enables the flag only when a
document reports a genuinely new selection. The flag persists in the View state;
the handler and checked provider operate on the app-wide state and apply to every
canvas. `Extras` still overrides it.

Alternatives: reuse `selection_revision` (rejected: it re-shows ants on in-place
edits, moves, Undo and Quick Mask); reset the flag in every selection
command/tool (rejected: scattered call sites); keep it persistent (rejected:
contradicts the documented per-selection behavior).

### One shared overlay refresh computes effective visibility

The frame applies the effective visibility to every canvas (the existing
`forEachCanvas` pattern). `ToolController` keeps refreshing the active canvas's
data, and the frame pushes the same data and flags to sibling canvases. The
active tool still contributes its editing decorations (slice selection/drag,
path anchors/handles, note current marker) and may override only the per-item
enabled term. The effective flag is `enabled && (extras || !member)`; Extras is
never overridden. `ImageView` paints only what its flags allow.

Alternatives: keep the refresh in `ToolController` and write only `canvas_`
(rejected: sibling canvases never receive the overlay); let `ImageView` read the
session directly (rejected: the bridge does not expose the session and would
couple paint to menu state); gate each paint call at the call site (rejected:
duplicated logic and easy to miss).

### Grid uses document-space spacing; Pixel Grid ignores the grid settings

The Grid overlay receives the interval in document pixels for each axis
(Percent divides width and height evenly), the subdivisions, colour and style,
and paints only inside the visible document rect. Pixel Grid is independent: a
one-document-pixel grid drawn when `zoom > 5.0`, with no preferences. Both
follow pan, zoom and view rotation.

### Grid preferences become real controls

The existing Grid group is a disabled placeholder whose interval and
subdivision values are labels. Replace the labels with numeric/spin inputs, give
the style and unit combos object names and signals, default to Light Gray /
Lines / 1 inch / 4, persist the keys through the guide view state, and apply a
change to every canvas.

Alternatives: keep fixed defaults and defer the controls to #505/#506 (rejected:
the Grid is unusable without a configurable interval and colour).

### Layer Edges cache key

Use `transform_api::contentRect` (backed by `layer_content_bounds_extents`) and
cache the rect until the active target changes or the exposed
`canvas_revision()` changes. Do not use `document_generation()`, which changes
only on load or replace.

Alternatives: recompute during every paint (rejected: a full content scan per
frame); use `Layer::rect` only (rejected: it includes transparent padding and
would not outline the content).

### Target Path reuses the existing builder

`workPathOverlay(*v, -2)` already reads the Paths panel's active path and
returns a curve-only overlay; wire it into the shared refresh instead of
generalizing the builder. A shape-layer path is readable only while
`path_on_layer` is on, so reuse the Paths panel's temporary
`path_set_layer_target` read or add a small bridge. The Pen, path selection and
shape tools keep their own overlay with anchors and handles, and the base curve
is suppressed while a path-editing tool is active. The curve refreshes on
`PictureView::changed` and a Paths panel target change.

Alternatives: reuse the tool overlay wholesale (rejected: anchors/handles are
editing affordances, not the Target Path extra); add a Rust-side curve encoder
(rejected: the subpath/point bridge already exposes the data the C++ overlay
builder needs).

### Slice visibility moves out of the Slice tool

The Slice tool no longer owns the base overlay: the shared refresh resolves the
document's slices whenever the canvas refreshes, and the tool only adds the
selection highlight and the live drag. `Show > Slices` gates the base overlay
for other tools. `Lock Slices` is a `ToolController` flag (like
`guidesLocked_`) that makes both slice tools inert. `Clear Slices` is a new
engine operation plus a bridge call that records one `Edit::Meta` state.

Alternatives: leave the overlay tool-owned and force Show on while the tool is
active (rejected: it cannot show slices with another tool); duplicate the
overlay builder in the tool and the shared refresh (rejected: two sources of
truth).

### Zoom presets share the existing behavior

Add `ImageView::printSize(ppi)` (or an equivalent small helper) and call it from
both the View command and the zoom tool context menu. `200%` reuses `setZoom`.
Both commands get enable providers on `has_document`.

Alternatives: duplicate the eight-line print-size calculation in the command
handler (acceptable but unnecessary); change the context menu to dispatch the
View commands (rejected: the context menu must keep working without the registry
in tests).

### Catalogue and menu layout changes

Flip `implemented` for the wired rows and `checkable` only for Extras, the ten
Show entries and Lock Slices (the zoom presets, All, None, Clear Slices and Show
Extra Options stay non-checkable); add identifiers for the new commands,
regenerate `command_ids.h`, delete the `View > Align` and `View > Align To` rows
plus their separator, and place `Clear Slices` after `Lock Slices` and
`Show Extra Options` at the end of the Show submenu. The edition-gated
`view.show.3d.axis` and `view.show.count` stay extended-only. The catalogue
golden (`catalogue.tsv`, `const_names.tsv`) and its counts (679 rows, 483
implemented, 403 const names) are updated in the same task.

Alternatives: keep Align/Align To disabled (rejected: the docs record that they
are not Photoshop menu items); add Clear Slices without a docs update (rejected:
the docs are the contract and the change carries `TASK-ALLOWS-DOCS`).

### Persistence and migration

Add the new fields with the same `value(...).toBool(default)` pattern already
used by `session.cpp`. Absent keys take the CS6 defaults, so no version bump or
migration step is needed. The membership list is stored as an array of command
ids; an unknown id is ignored on load.

Alternatives: bump the session schema (unnecessary for additive defaults);
store the whole state as opaque JSON (harder to test and to read in the session
store).

## Risks / Trade-offs

- **Layer Edges cost** → compute on target changes and `canvas_revision()`
  changes, cache the rect, and never scan content during paint.
- **Grid line explosion at tiny intervals** → draw only the lines intersecting
  the visible document rect and clamp the minimum spacing to one screen pixel.
- **Selection generation can be confused with in-place updates** → add a
  replacement-only generation, store it per document view, and verify with a
  selection command, a tool drag, an in-place edit, a move, Undo and Quick Mask
  that the ants reset only for a genuinely new selection.
- **Membership semantics are inferred from Adobe prose** → the spec records the
  chosen model, the design notes the alternative, and the tests pin it.
- **Samplers have no Show entry** → give them their own flag and make All/None
  set and clear it, so Extras remains the only other gate.
- **Persistent paths/slices may duplicate tool overlays** → one shared base
  overlay plus tool-owned decorations, suppress the base Target Path curve for
  path-editing tools, and test that the Pen shows anchors without a duplicate
  curve while Target Path is on.
- **Sibling canvases can diverge** → apply the state and overlay data to every
  canvas, and test two canvases for slices, notes and target path.
- **Edition differences** → Standard has nine Show entries and an
  application-global Show Extra Options window; Extended adds Count. Test both
  editions for the window and the Count overlay.
- **Test suite size** → keep `tst_view_menu.cpp` under the test cap by moving
  shared helpers to `tst_view_menu_cases.h` or splitting overlay cases into a
  second registered suite if needed.
- **Existing scenarios change** → update the slice-tool, count-tool, note-tool,
  smart-guides, rulers, selection and edition-gating suites rather than
  weakening their old scenarios.

## Migration Plan

No user-data migration. The change is additive session state plus command-table
edits; a rollback simply ignores the new keys. The PR must archive the change
with `openspec archive` in the same pull request per the zero-open rule, and
update `ROADMAP.md` and the parity register.

## Open Questions

None that affect the specs or the task breakdown.
