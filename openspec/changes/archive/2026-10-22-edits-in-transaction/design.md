# Design

## Context

The edit transaction lives in `crates/pictura-app/src/cxxqt_object/edit.rs`. A
commit names a history label and a mutation; the transaction composites, clears
the positional link sets when the change invalidates them, captures one history
state, advances the content revision and marks the document dirty. The mutation
declares one of six outcomes, and the outcome decides both the compositing and
the notification:

| outcome | composites | captures history | notifies (`changed`) |
|---|---|---|---|
| `None` | no | no | no |
| `Meta` | no | yes | yes |
| `Record` | no | yes | no — a preview already presented it |
| `Regions { notify }` | the rectangles | yes | only when `notify` |
| `Full` | the whole document | yes | yes |
| `Structural` | the whole document, links cleared | yes | yes |

The shell receives that notification as a signal: `frame.cpp` connects
`PictureView::changed` to `PicturaMainWindow::refresh`, which rebases the
canvas, syncs the overlays, updates the status bar and calls
`CommandRegistry::refresh()` — about 140 enablement, checked-state and label
providers, each a crossing into the Rust bridge. The frame's `refresh()` also
arms the 120 ms `panelRefreshTimer_`, so the panels follow later; it never
refreshes them synchronously.

Two facts shape the approach.

**The outcome already decides whether the shell was presented.** A commit whose
outcome notifies has been presented; a commit that did not (a bounded region
with `notify` false, or `Record`) has not. The shell therefore does not need a
new interface to know whether its own `refresh()` is redundant — it needs to
know *whether the command's committed outcome always notifies*.

**Some commands commit outcomes of which only some notify.** `apply_filter`
commits `Regions { notify: false }` when the layer carries no effect and
`Full` when it does; `set_layer_visible` commits `Regions { notify: false }`
when the layer's reach is boundable and `Full` when it is not. For those
commands the shell cannot decide locally, and the transaction's outcome has to
reach it across the bridge. That is a separate change.

This change takes the families whose committed outcome *always* notifies, which
is where the redundancy is unconditional and can be removed without a new
interface.

## Goals / Non-Goals

**Goals:**

- The transaction brackets the mutation for the layer-tree family: the change
  and its history state are created together, and a refused mutation writes
  nothing.
- A committed edit presents the change to the shell once.
- A command handler does not recompute command state a second time for the
  same notifying committed edit.

**Non-Goals:**

- Reporting the committed outcome across the bridge. The mixed-outcome
  families — filters, image adjustments, replace and match colour, clipboard
  paste and clear, the single-layer property and opacity/fill commits, the
  layer-visibility eye toggle — keep their shell `refresh()`. Their shell files
  are `frame_menus_filter.cpp`, `frame_menus_adjust.cpp` and the
  clipboard, property and visibility handlers of `frame_menus_edit.cpp` and
  `frame_menus_select.cpp`.
- A transaction outcome for "a bounded region that also invalidates the
  positional links". Two of the four sites that still refresh and record by
  hand need it; they wait for it.
- Migrating the remaining mutation-outside-the-closure call sites. They are
  migrated one family per change.
- The transform and mode menus (`frame_menus_transform.cpp`,
  `frame_menus_image_mode.cpp`). They start a session or re-sync a checkable
  action; they commit no document edit of their own, so their `refresh()` is
  not a second presentation.

## Decisions

### The layer-tree family is this change's slice

Every layer-tree command commits an outcome that always notifies:

| command | bridge site | outcome |
|---|---|---|
| arrange, reverse, stamp | `layer_arrange.rs` | `Structural` |
| lock all layers in group | `impl_layers/layers_surface.rs` | `Full` |
| lock layers | `impl_layers.rs` (`mutate_meta`, `batch_property`) | `Meta` |
| merge, merge visible, merge clipping mask, flatten, background conversions | `impl_layers_merge.rs` | `Structural` |
| new layer, new group, group from layers, layer via copy/cut | `impl_layers_create.rs` | `Structural` |
| delete hidden layers | `impl_layers_select.rs` | `Structural` |
| hide layers | `impl_layers_select.rs` (`batch_changed`) | `Full` |
| rasterize fill content / layer / all / type, add solid and gradient fill | `impl_layers_rasterize.rs` | `Structural` |
| rasterize shape | `shapes.rs` | `Structural` |
| convert to smart object, place, replace and edit contents, rasterize smart object | `impl_layers_smart_object.rs` | `Structural` |
| reset transform | `impl_layers/smart_object_actions.rs` | `Structural` |
| add, delete, apply, enable, disable, link and unlink a layer mask | `impl_layers/layer_masks.rs` | `Full` |
| add, delete, enable, disable, link, unlink and rasterize a vector mask | `impl_layers/vector_masks.rs` | `Full` |
| create and release a clipping mask | `clipping.rs` | `Full` |
| align, align to selection, distribute | `align.rs` | `Full` |
| paste, clear, scale layer style, show and hide all effects | `layer_style.rs` | `Full` |
| remove black and white matte, defringe | `impl_layers/matting.rs` | `Full` |

The shell's `refresh()` after such a command is therefore unconditionally
redundant, and no new interface is needed to remove it.

The layer mask, vector mask, clipping and align commands already run their
mutation inside the closure through `batch_changed` in `impl_layers.rs`; the
lock properties use `batch_property`. They need no Rust change, only the
shell-side deletion. The other sites still mutate outside the transaction and
move in.

Three layer-tree commands are not document edits and need no deletion:

- **Link and unlink layers** (`impl_layers_select.rs:54`) touch the transient
  link map and commit no edit at all; their handlers do not even refresh.
- **Select similar, select linked, select all layers and deselect layers** are
  read-only or session-state only.
- **Copy layer style, copy shape attributes and the clipboard copy commands**
  commit nothing.

**Alternative considered:** migrate every family and add a bridge accessor that
reports whether the shell was presented. Rejected here because it enlarges the
bridge interface for a decision only the mixed families need, and those
families are a separate change with their own review.

### The mutation moves into the closure, and the closure reports what it did

The target shape is the one the codebase already uses in `mutate_layer`,
`mutate_meta`, `batch_changed` and `batch_property`:

```rust
// before: the mutation happens outside the transaction, and the outcome is
// asserted afterwards.
let landed = { /* mutate the document */ };
view.rust_mut().select_only_layer(&landed);
view.as_mut().edit(label, |_| Edit::Structural);

// after: the transaction brackets the mutation.
view.as_mut().edit(label, |rust| {
    let Some(doc) = rust.session.doc.as_mut() else {
        return Edit::None;
    };
    let Some(landed) = arrange_path_index(doc, &path, op) else {
        return Edit::None;
    };
    rust.select_only_layer(&landed);
    Edit::Structural
});
```

The closure receives `&mut PictureViewRust`, which is the same handle the
mutation had before, so nothing needs the QObject while it runs. A closure MUST
NOT call `edit` again: it calls engine operations and target/session helpers
only. The transaction takes no snapshot and performs no rollback; it records a
state only when the closure reports a change, so a closure MUST check its
refusal before it writes. The engine operations the migrated sites call document that a refusal
leaves the document untouched; each migrated closure checks the result before
it selects a target or returns an outcome. A command that
has to hand a value back to its caller captures it in a mutable local, the
idiom `add_layer`, `duplicate_layer` and `group_layer` already use:

```rust
let mut created = -1;
self.as_mut().edit("New Layer", |rust| {
    let Some(doc) = rust.session.doc.as_mut() else {
        return Edit::None;
    };
    created = pictura_render::add_layer(doc, above, &name);
    if created >= 0 {
        rust.layer_target.select_only(&created.to_string());
        Edit::Structural
    } else {
        Edit::None
    }
});
created
```

**Alternative considered:** widen the transaction's closure to return the
outcome and a value together. Rejected: the capture idiom is already
established, and a second return type would touch every migrated call site
without buying anything.

### `Edit::Record` means "a preview already presented it"

`Record` captures history without a notification, and its only legitimate use
is the commit that finishes a preview the shell already showed. Every site
that uses it today:

| site | verdict |
|---|---|
| `impl_layers/mask_refine.rs:81` | legitimate — it records a preview another control left uncommitted, under that control's own label |
| `layer_style.rs:220` | legitimate — `layer_style_commit` finishes the Layer Style dialog's live preview |
| `adjustment_edit.rs:197` | legitimate — `adjustment_commit` finishes the adjustment panel's live preview |
| `impl_selection.rs:392` | misuse — `save_selection` adds a channel nothing previewed |
| `channels.rs:149` and `channels.rs:165` | misuse — New Channel and Delete Channel change the channel list nothing previewed |
| `impl_layers.rs:706` and `impl_layers.rs:770` | dead — see the decision on `changed_property_region` |

The three misuses become `Edit::Meta`: the change touches no composite pixel,
so nothing is composited, and the shell is notified. `save_selection` therefore lets the
Select menu's own `refresh()` go; the Channels panel keeps its own `refresh()`
either way, because that is the panel's immediate update and the frame's panel
refresh is deferred by the 120 ms `panelRefreshTimer_`.

**Alternative considered:** leave the three misuses alone and keep their
callers' `refresh()`. Rejected: it leaves a documented channel the shell is
never told about, and it keeps two refreshes where one will do.

### The unreachable `Some(None)` branches fold away

`changed_property_region` (`impl_layers.rs:26`) returns
`Option<Option<PsdRect>>`. Its outer `None` means "a changed path has no
boundable region", and its `Some(None)` means "no path's
`(blend, opacity, fill)` tuple changed". Every caller reaches it only after the
operation reported a non-zero change count (the previews guard on `changed >
0`), and `set_blend_paths`, `set_opacity_paths` and `set_fill_paths` count a
node only when its value actually changed, so at least one path always differs.
`Some(None)` is therefore unreachable, and the three branches that match it are
dead code: `impl_layers.rs:1004` would skip the composite, and
`impl_layers.rs:706` and `impl_layers.rs:770` would mark a changed edit as
already presented. They fold into the existing `_ => Edit::Full` fallback so
the match stays exhaustive and a changed edit always composites and notifies.

**Alternative considered:** keep the branches as a defensive default. Rejected:
a defensive branch that silently skips the composite or the notification is
worse than no branch.

### The redundant shell `refresh()` calls go, and nothing else in the shell changes

The audit rule is mechanical: for each command handler, look at the outcome its
command commits. If every successful outcome that command can commit notifies, the
handler's `refresh()` is a second presentation of the same change and is
deleted. If one of them does not notify, the handler keeps it.

```
handler:  if (view && op(*view)) { refresh(); }
                    |
                    +-- every successful outcome op can commit notifies
                    |     (Meta, Full, Structural, Regions{notify:true})
                    |     -> delete the refresh(); the signal already presented it
                    |
                    +-- op can commit a successful outcome that does not notify
                          (Regions{notify:false}, Record)
                          -> keep the refresh(); the handler remains responsible for presentation
```

The handlers keep their shape. Nothing new is added to the C++ side. The rule
applies to `PicturaMainWindow::refresh()`; a panel's own `refresh()` — the
Channels panel's, the Layers panel's — is the panel's immediate update and
stays.

### The five double notifications go

`edit()` notifies for `Meta`, `Full`, `Structural` and `Regions { notify: true }`.
Five bridge sites emit `changed` again immediately afterwards, so a view
wired to the frame was presented twice by the bridge; Trim and Canvas Size
also refreshed from their handler:

| site | operations |
|---|---|
| `image_adjust/image_ops.rs:83` (`finish`) | Image ▸ Trim, Image ▸ Duplicate |
| `image_adjust/image_size.rs:167` | Image ▸ Canvas Size |
| `type_tools.rs:926` | Apply Type Style (Paragraph Styles panel) |
| `type_tools.rs:947` (`commit_style`) | New/Edit/Delete Character Style, New/Edit/Delete Paragraph Style |
| `type_tools.rs:1130` (`commit_edit`) | Edit Type Layer (Type tool commit, Character/Paragraph panels) |

Five sites, eleven operations: `finish` serves both Trim and Duplicate, and
`commit_style` serves the six style commands. The extra emission is deleted.
The transaction's own notification is the one that remains. The type operations
are API calls from panels and the Type tool, not command-registry entries, so
they carry no handler refresh; Image ▸ Duplicate emits both notifications on
the new view before `addDocument` wires it to the frame.

**Alternative considered:** keep the extra emission and delete the handler's
`refresh()` where one exists. Rejected: two presentations of one change are
still two, and the transaction is the only place that knows whether a change
was presented.

### The command registry's refresh count is the observable

`CommandRegistry::refreshCount()` already exists as a test probe, and
`selftest_paint_perf.cpp` uses it to assert that a stroke release runs the
registry exactly once. The new suite uses the same probe for the shell's
refresh: a command that commits one edit must move the counter by one before
`dispatch` returns. The 120 ms `panelRefreshTimer_` can refresh panels and
recompute command state later; that is a separate trigger, not a second
presentation of the edit, and the test reads the counter synchronously.

A registry command is dispatched programmatically through
`CommandRegistry::dispatch(id)`, as `tst_command_tree` does, and not by opening
a menu: `CommandRegistry` refreshes on `QMenu::aboutToShow` as well, and a
menu-driven dispatch would count that extra run. The type-style operations are
`type_api` calls, so the suite invokes them directly.

A Qt Test drives a real command through `PicturaMainWindow::registry()` and
counts. The Rust side gets unit tests for the transaction semantics: a mutation
that reports no change commits no history state, and the outcome the closure
reports is the one the transaction returns.

The test uses a command whose handler only calls the view (`Layer ▸ Flatten
Image`), not one that also refreshes the Layers panel: a panel refresh emits
`selectionChanged`, which recomputes the command table as a separate
panel-driven trigger. Counting that path would blame the handler for a
recomputation the spec deliberately excludes.

**Alternative considered:** count `changed` emissions with a `QSignalSpy`, as
`tst_document_session.cpp` does. That proves the bridge notifies once but says
nothing about the shell refreshing twice, which is the defect. The registry
count is the direct witness, so both are used: the spy for the bridge, the
counter for the shell.

### The undo label names the state it will undo

The transaction records the history state before it emits `changed`, so the
registry refresh that follows already sees the new cursor. The Edit ▸ Undo
label provider in `frame_menus_edit.cpp` reads the label at that cursor (the
edit just committed), and Edit ▸ Redo reads the label one step ahead after an
undo. Before this change the provider always returned the bare "Undo"/"Redo",
so the command table could not satisfy the "label names the edit it follows"
scenario without another refresh.

## Risks / Trade-offs

- **A deleted `refresh()` also served a non-document purpose.** Some handlers
  refresh after a command that also updates a dialog or a status field. The
  audit is per handler and the rule is narrow: only handlers whose command
  always notifies are touched. → The new suite asserts the refresh
  count for the affected commands, and the manual checklist in the pull
  request covers the visible result of each migrated family.

- **The frame's panel refresh is deferred, the handler's was not.** The
  handler's `refresh()` also armed the 120 ms timer, so the panels' timing is
  unchanged; the panels never refreshed synchronously from it. → The Channels
  panel keeps its own `refresh()` calls, and `tst_channels_panel` covers the
  immediate update.

- **A migrated mutation now runs inside the transaction's closure**, which
  returns early when there is no document. Previously the mutation ran first
  and the early return came after. → The mutation is a no-op without a
  document in every migrated site, so the observable result is unchanged; the
  Rust tests cover the no-document path.

- **The closure borrows the whole `PictureViewRust`.** A migration that reads
  both the document and the selection — a fill layer's default mask, for
  instance — must snapshot the selection before it takes the document borrow.
  → Each migrated site keeps the borrow order it already has; the compiler
  enforces it.

- **`frame_menus.cpp` is at 1121 lines, under the 1200 cap.** The deletions
  bring it down. → The migrated families must not push it back up; the
  remaining families migrate with their own change.

- **The four hand-written commit sites stay as they are**, so a document change
  can still be presented before its history state exists. `refresh_edit`
  (`paint_tools/fills.rs:480`) emits `changed` in both of its branches, so the
  fill and shape paths notify — before they record. → They are named in the
  proposal as the next change's work, with the outcome they need.

- **Removing the shell's `refresh()` makes the transaction's notification
  load-bearing.** A command whose outcome stops notifying would silently stop
  refreshing the shell. → The Rust tests assert the outcome of the migrated families' representative
  commands, and the Qt suite asserts the registry count for representative
  commands from each family.

## Migration Plan

None. No stored data, file format, bridge signature or dependency changes. The
change is a source-level refactor plus the removal of redundant refreshes; a
revert restores the previous behaviour exactly.
