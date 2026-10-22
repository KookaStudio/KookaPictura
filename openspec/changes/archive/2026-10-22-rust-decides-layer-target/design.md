# Design

## Context

The layer target lives in Rust (`crates/pictura-app/src/layer_target.rs`) as the
selected paths plus the active path, one per open document, and the `PictureView`
QObject is the adapter that presents it. The bridge
(`cxxqt_object/layer_target_bridge.rs`) exposes it to C++ as
`selected_layer_paths`, `layer_selection_anchor`, `select_layer_path`,
`set_layer_target` and `clear_layer_target`. The Layers panel is a projector
that already reads the target (`LayersPanel::applyTargetSelection`,
`layers_panel.cpp:785`) and writes it when the user picks rows
(`LayersPanel::writeTarget`, `layers_panel.cpp:767`).

Three things sit in the way of that projector contract.

**The engine reports the wrong copies.** `pictura_render::duplicate_paths`
(`document_ops/layer_ops/properties.rs:276`) returns each copy's path as it was
at insert time. The copies are inserted topmost-first, so every later insert
lands at a lower index and pushes the earlier copy up one: only the last copy's
path survives. The layer stack is bottom-first (`doc.layers[0]` is the bottom
layer), so the stale path of an earlier copy names an original layer instead.

**The target rules are unreachable from a Rust test.** The rules live in
`impl qobject::PictureView` methods, which take `Pin<&mut PictureView>` — a
cxx-qt object a plain `#[test]` cannot construct. The testable half of the
view is `PictureViewRust` (`cxxqt_object/state.rs`), which `document_session/tests.rs`
and the app's Rust tests drive directly.

**The panel's per-document state is per-panel.** `LayersPanel` is one widget
shared by every document, and its `soloSnapshot_`, `isolatedPaths_` and
`expandedPaths_` are members of it. `setView` (`layers_panel.cpp:448`) clears
the solo and the isolation on a document switch but not the expansion. Ending
isolation there is required (`ui/isolation-mode`); losing the solo snapshot is
not, because the document stays soloed with nothing able to restore it.

The repo's rule is that Rust is the engine and the state holder and Qt/C++ is
the representative layer, so every decision that outlives a repaint belongs in
Rust.

## Goals / Non-Goals

**Goals:**
- One writer for the target: the engine. The panel's row gesture is the only
  exception, because the user is picking the rows there.
- The target rules reachable from a Rust test, so a regression in them fails
  `cargo test` and not a Qt suite that needs a window.
- The engine's copy paths correct, so the duplicate target names the copies.
- The panel's solo snapshot and expansion belong to the document. The isolated
  set stays transient and a document switch ends it, as `ui/isolation-mode`
  requires.

**Non-Goals:**
- Which layer an operation selects does not change, except that a duplicate
  now selects every copy, an adjustment creation now selects the created layer,
  and Select Similar keeps its active layer, matching the `Select` menu's
  existing path.
- The layer dialogs are not redesigned, and the Layer menu's handlers stay in
  C++; only their post-op selection goes.
- The Select All/Deselect target writes stay where they are: changing the
  selection is their purpose, and both go through the Rust bridge.
- The history's target capture is untouched: `history.rs` already carries the
  pre- and post-operation target of every state and restores them correctly.
- The Move tool's auto-select, the filter's dimensions and the row menu's
  command set are untouched.

## Decisions

**1. The engine reports every copy at its final path.** `duplicate_paths`
resolves and clones every selected source against the pre-operation document
before the first insert, so an earlier copy cannot hide a source that comes
later. It then inserts the copies in panel order (topmost-first, the order
`flatten_rows` returns), so an insert never moves a source that has not been
copied yet and the source's pre-operation parent and index are still valid.
Finally it replays the insertions to compute each copy's final path: an insert
at index `i` in a container shifts a copy's index by one when the container is
the copy's parent and the insert is at or below the copy, and shifts the copy's
ancestor segment by one when the insert lands in that ancestor's container at
or below the ancestor. The returned paths are the copies' real paths, in
processing order, and the target that names them names the copies.
   - *Alternative considered:* report the paths by resolving the copies after
     the fact — rejected: the stack has no stable identity for a node, so the
     copies cannot be told from the originals after the fact.
   - *Alternative considered:* keep the removal order `selected_paths` uses
     (descendants and higher-index siblings first) — rejected: that order puts a
     lower-index root layer before a higher group's child, so inserting the
     root copy shifts the child's parent and loses the child. Panel order keeps
     every not-yet-copied source's parent in place.

**2. The target rules move onto `PictureViewRust`.** Each structural operation
gets a named rule there — `duplicate_target`, `merge_target`, `group_target`,
`ungroup_target` — and the bridge methods call them. A `PictureViewRust` is
constructible in a test (`document_session/tests.rs` already does it), so the
rules are testable without a window. The ungroup rule is the only large one:
its index arithmetic moves out of the edit closure into a function over the
pre-operation group/survivor data and the post-operation document.
   - *Alternative considered:* keep the rules in the bridge and test them
     through the Qt suites — rejected: a Qt suite needs a full
     `PicturaMainWindow`, which is why the bug survived a green suite.
   - *Alternative considered:* move the whole operation onto `PictureViewRust`
     so the test drives the real op — rejected for now: the bridge's `edit`
     transaction owns the compositing and the history capture, and moving the
     op would split the transaction in two. The rules are the part worth
     testing, and they are the part that was wrong.

**3. A target write outside an edit updates the history's belief.** `history.rs`
keeps `current_target`, the target it believes is live, and `capture_live` uses
it as the pre-operation target of the next state, then sets it to the
post-operation target. A rule that runs inside `edit` therefore must not touch
the belief: `record` already updates it. A selection-only rule (Select Similar,
Select Linked) does not run an edit, so it must call `history.set_current_target`
exactly as the bridge's `set_layer_target` does; otherwise the next edit records
a stale pre-operation target.
   - *Alternative considered:* have `PictureViewRust::set_layer_target` always
     update the belief — rejected: it is called by rules inside the edit
     closure, before the state is captured, which would overwrite the
     pre-operation target with the new one.

**4. The panel remembers the target it agreed with.** Two members hold the
selected paths and the active path the panel last saw, updated when the panel
writes the target and after a refresh has projected it. A refresh compares the
live target with the remembered one before updating it: when the target moved,
the panel's current row becomes the target's active path (when that row is
displayed); when it did not, the pre-refresh current row is restored (when it is
still displayed), which keeps a current row the user moved with Ctrl+arrow. When
the current row moves outside the selection, the panel keeps the target's
existing active path rather than resetting it to the first selected path, so the
stray move is not mistaken for a target change. The remembered target is cleared
on a view or document-generation change, so a shared panel never compares one
document's target with another's. This is what makes the deletion of the C++
post-op selection safe: the old current row can be outside the new target (for a
single-layer duplicate it is the source path), and without the rule the refresh
would snap it back instead of showing the operation's result.
   - *Alternative considered:* always take the current row from the target —
     rejected: it would undo the Ctrl+arrow behaviour the panel supports
     today.
   - *Alternative considered:* compare the current row with the target's active
     path instead of remembering the target — rejected: the two cases are
     indistinguishable in that comparison, because in both the current row is
     outside the selection.

**5. The commands' enablement reads the target.** The Duplicate provider
(`frame_menus.cpp:340`) reads the tree's rows; the Align and Distribute
providers already read `PicturaMainWindow::selectedLayerPaths()`
(`frame_menus_align.cpp:22`), as does every other provider. The tree's rows and
the target disagree whenever the panel has not been refreshed, so the Duplicate
provider moves to the target like the rest.
   - *Alternative considered:* make the panel's rows and the target agree by
     construction — rejected: the panel refreshes on a queued signal, so there
     is a window in which they differ, and the providers must not depend on
     the panel having run.

**6. The solo snapshot and the expansion move into `PictureViewRust`.** They
become fields beside the existing `isolation`, cleared with the rest of the
document-scoped state (`document_session/session.rs:387`). Each open document
has its own `PictureView` and its own `PictureViewRust` (`docs_[]` in
`frame.h`), so the state is per-document by construction. The bridge gains
`solo_set`, `solo_restore`, `solo_clear`, `solo_active`, `expansion_set` and
`expansion_paths`. The existing `isolation` field is not moved: it stays the
selected-filter scope, which a document switch clears as `ui/isolation-mode`
requires.
   - *The solo's rules move to Rust with it.* The soloed paths are the clicked
     layer, its ancestors, and, for a group, its descendants that were visible
     in the snapshot; the snapshot is every layer's visibility.
     `pictura_render::apply_visibility` already makes exactly the listed paths
     visible, so the engine has everything it needs. `solo_set` and
     `solo_restore` each apply their visibility change as one edit with the
     existing `"Solo Visibility"` / `"Restore Visibility"` labels.
   - *A structural edit clears the snapshot in the engine.* `apply_edit` drops
     the solo snapshot when it commits `Edit::Structural`, so every structural
     path discards it even when a C++ caller forgot `clearSolo()`.
   - *Adjustment creation clears it explicitly.* `add_adjustment` records a full
     or region recomposite, not `Edit::Structural`, so it calls the same
     solo-clear helper itself; otherwise a solo would survive the new layer.
   - *The snapshot's lifetime moves with it.* Today the snapshot is dropped
     when the active document changes, which leaves the document still soloed
     with nothing able to restore it. Per document, the snapshot survives the
     switch and the second Alt-click still restores.
   - *The mask solo view stays its own field.* A document switch still ends the
     mask solo view on the outgoing document (`ui/mask-solo-view`), but it no
     longer clears the layer solo snapshot.

**7. The expansion's name-chain re-find stays in the panel.** The panel
re-finds an expanded group whose index path shifted by its ancestor name chain
(`layers_panel.cpp:596`), because the model — which knows the names — is in
C++. The panel therefore keeps the re-find and writes the remapped set back to
Rust after each refresh; Rust holds the set, the panel maintains it. The
alternative — moving the name chain into Rust — would need the name chain to
survive a rename, which the panel's path-based set does today for free.
   - *Alternative considered:* store the expanded set in Rust by name chain —
     rejected: a rename would drop the expansion, which
     `lpr_rename_keeps_expanded` forbids.

**8. The panel's new code goes into its own file.** `layers_panel.cpp` is at
1132 lines against a 1200 cap, and this change adds a rule and a set of bridge
calls to it. The projector — the target-moved rule, the remembered target, the
expansion's read-back — moves into a new `panels/layers_panel_target.cpp`, and
`CMakeLists.txt` names it explicitly, as the repo requires for every new file.

## Risks / Trade-offs

- [The ungroup rule moves 55 lines of index arithmetic] → It moves into a pure
  function over the pre-operation group/survivor data and the post-operation
  document, and it gets its own Rust tests before any C++ deletion, so a
  mistake fails `cargo test` and not a window.
- [The target-moved rule touches every refresh, and the panel is used by about
  twenty Qt suites] → The rule changes one branch in `refresh()`, and the
  panel's existing target tests (`tst_layer_target`) cover the selection and
  the current row; the new cases cover the moved and unmoved target.
- [The solo's snapshot now outlives a document switch] → This is the specified
  behaviour change, and it is a fix: today the document stays soloed with the
  snapshot dropped. The self-test's solo case (`ST_FAIL` code 106) keeps
  covering the visibility, the undo and the restore.
- [The engine's copy paths change] → The engine test's golden moves with them,
  and the change records the old and the new paths so the difference is
  visible in review.
- [A per-document solo means a document can come back still soloed] → The
  second Alt-click restores it, and the solo's own visibility is one undoable
  step, so the user is never stuck with an invisible layer.
- [The expansion is written back by the panel, so Rust is not the only writer]
  → The write-back is the panel's name-chain re-find, which the model needs;
  the *set* lives in Rust and is what a document switch reads.
