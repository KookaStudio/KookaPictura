# Design

## Context

See `proposal.md` — Why. The current row-menu path:

```
PictureView (Rust bridge)
    |  layers_api::snapshot
    v
QVector<LayerRowNode> ---> LayersModel (QAbstractItemModel, 936-line header)
                                |
                                v
                    proxy -> LayersTreeView -> LayerRowDelegate

LayersPanel::populateRowMenu
    |  kRowSpecs[50] { id, label, kinds, block, implemented }
    |        |
    |        v
    |  commandForRow(): 134 lines, 40 static id->command returns,
    |                  5 dynamic branches, 8 unimplemented rows
    |        |
    |        v
    |  CommandRegistry::dispatch(commandId)
    |
    +--> live facts: layer_is_fill_content,
                     layer_can_rasterize_smart_object,
                     clipping_can_release
```

`LayersModel` already stores every `LayerRowNode` and indexes them by path
(`byPath_`), derives `clipBase`, and is constructible without a `PictureView`
(`setView` is only needed for rename writes). The model rename is 5 references;
the class has no `Q_OBJECT`, so no moc unit changes.

## Goals / Non-Goals

**Goals:**

- One place — a `RowAction` table — maps a row-menu item to its command id.
- Menu construction reads only a row (`LayerRowNode`) plus three on-demand
  command-availability facts. No `PictureView` call inside the menu logic.
- A Qt test constructs `LayerRowModel`, sets hand-built rows, and asserts the
  resolved actions without `PicturaMainWindow` or a document.
- `populateRowMenu` keeps its block order, separators, labels, enabled states,
  tooltips, and the `CommandRegistry` dispatch path.

**Non-Goals:**

- Moving the 175 `*ForTest` hooks out of `LayersPanel` (the adapter follow-up).
- A `LayerCommandTarget` interface or moving the anchor/target write; #829
  owns target ownership and `CommandRegistry` already dispatches by id.
- The panel-flyout table (`rowsForPanel` + `panelCommandId`), the Background
  brief menu, the type-row submenus, and the mask child menus.
- Any Rust bridge or snapshot change.

## Decisions

### The row model is the existing `LayersModel`, renamed

`LayersModel` already owns the rows and the path index; a second
`LayerRowModel` holding the same vector would duplicate state and need a sync
rule. Rename the class to `LayerRowModel` and add:

```cpp
const LayerRowNode* rowForPath(const QString& path) const;
QVector<ResolvedRowAction> actionsFor(const QString& path,
                                      const RowFacts& facts) const;
```

The actions method is a lookup plus `resolveRowActions(row, facts)`. The pure
resolver lives in a new `layers_row_actions.{h,cpp}` so the table and the
resolver can be unit-tested without a `QAbstractItemModel`.

Alternative: a new standalone `LayerRowModel` class. Rejected: two owners of
the same row vector, and `LayersModel` would still be the Qt view's model.

### Menu facts are evaluated on demand, not stored per row

The three live predicates are command availability for one clicked row, not
display state. Add:

```cpp
struct RowFacts {
    bool fillContent = false;
    bool canRasterizeSmartObject = false;
    bool canReleaseClipping = false;
};
```

`populateRowMenu` fills it once from `view_` before resolving:

```cpp
facts.fillContent = view_ && view_->layer_is_fill_content(path);
facts.canRasterizeSmartObject = view_ && view_->layer_can_rasterize_smart_object(path);
facts.canReleaseClipping = view_ && clipping_can_release(*view_, {path});
```

`hasStyle`, `linked`, `hasMask`, and `hasVectorMask` are already snapshot fields
on `LayerRowNode`; no live call is needed for them.

Alternative: add the facts to the refresh `snapshot` (one crossing per
refresh). Rejected for this change: it would add three predicates — one of
which walks the clipping run — to every row on every refresh, on a path #1240
is already investigating, and would touch the Rust bridge and typed adapter
only for menu availability. If a second consumer needs them, promote them to
the snapshot then.

### One table with a command-id column

Replace `kRowSpecs` + `commandForRow()` with:

```cpp
enum class RowDynamic { None, RasterizeLayer, ClippingToggle, Link,
                        AddMaskSubmenu, AddVectorMaskSubmenu };

struct RowAction {
    const char* id;
    const char* label;
    unsigned kinds;        // the existing KindMask bits
    unsigned block;        // CS6 block order, including the implicit block 10
    const char* commandId; // commands.h constant for a static row
    RowDynamic dynamic;    // None for a static row
};
```

and the resolved, Qt-free shape the panel consumes:

```cpp
struct ResolvedRowAction {
    QString id;
    QString label;
    QString commandId;   // empty for a disabled stub or a submenu row
    int block = 0;
    bool enabled = true;
    bool submenu = false;
};
```

The resolver keeps the current predicates: kind mask (with `shape` forcing
`Pixel|Shape` and `smartObject` ORing `SmartObject`), `rasterizeStyle` hidden
without `hasStyle`, and `rasterizeLayer` hidden without `smartObject` or
`fillContent`. The five dynamic rows resolve to a command id from the row plus
`RowFacts`; the two mask rows set `submenu`.

Alternative: keep the chain and add a handler field (`RowAction { id, spec,
handler }` from the issue). Rejected: the handlers are already
`CommandRegistry` handlers keyed by command id; adding a second dispatch table
would duplicate the frame's registration. The table's job is the missing
id→command mapping, not execution.

### The panel keeps blocks, separators, and the bespoke menus

`populateRowMenu` groups resolved actions by `block` and walks blocks in the
current order (1–5, then type rows or 6, then 7–9, isolation, 11), so the
"one separator between non-empty blocks" rule and the type-row slot stay
identical. Rename, the Color Label submenu, the Background brief menu, the
type rows, and the mask child actions remain in the panel.

Alternative: put every row in the table, including type submenus and the
Background menu. Rejected: those are nested/ordered structures, not flat rows;
encoding them needs more code than the bespoke builders they replace.

### Label overrides stay a panel concern

The registry supplies most labels; nine ids (`rasterize*`, mask enable/disable/
delete) use the table label instead, exactly as the current code. Keeping the
override set at the QAction construction avoids changing registry labels and
keeps `tst_layers_row_menu` byte-identical.

## Risks / Trade-offs

- **A table row names the wrong command id** → `tst_layers_row_menu` asserts
  the texts, order, enabled states, separators, and command ids; it is the
  oracle and must not be edited. The static ids are copied from the current
  `commandForRow` branches.
- **`hasStyle` is a snapshot field, while `rasterizeStyle` previously read the
  live view** → the panel queues a refresh from `changed`, so the row is
  current by the next user event; the fx badge already relies on the same
  field. No observable difference in the suite.
- **The model rename touches a widely included header** → only 5 references,
  no `Q_OBJECT`, no moc unit; the compiler catches a missed forward
  declaration.
- **`actionsFor` returns pointers into the model's nodes** → the panel builds
  the menu synchronously and does not refresh during the walk; document this
  in the method comment.
- **File size** → `layers_panel_menu.cpp` loses the 134-line chain and keeps
  its 50-row table as data; the new `.cpp` is under the code cap; no allowlist
  growth.

## Migration Plan

One pull request, no runtime or data migration:

1. Add the test suite and test plan (red).
2. Add `layers_row_actions.{h,cpp}` and the table.
3. Rename the model and add `rowForPath`/`actionsFor`.
4. Rewrite `populateRowMenu` to resolve actions and delete the old lists.
5. Run the full gate set; rollback is a revert of the single change.

## Open Questions

None.
