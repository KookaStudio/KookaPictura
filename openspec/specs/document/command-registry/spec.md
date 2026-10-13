# command-registry Specification

## Purpose
A declarative command table keyed by stable identifier, the CS6 menu tree, and enablement recomputed on menu open.
## Requirements
### Requirement: Declarative command table
The system SHALL define every menu command once in a declarative table keyed by a
stable string identifier, carrying the menu path, label, default shortcut, and an
implemented flag. The menu bar SHALL be built from this table rather than from
hand-assembled `QAction`s.

#### Scenario: Menu bar is generated from the table
- **WHEN** the application frame is constructed
- **THEN** each top-level menu and child item corresponds to an entry in the command table

#### Scenario: Command identifiers are stable
- **WHEN** a command's label or shortcut is changed in the table
- **THEN** its identifier is unchanged and dispatch by identifier still resolves it

### Requirement: Full documented top-level menu tree
The system SHALL present the ten standard CS6 top-level menus -- File, Edit,
Image, Layer, Type, Select, Filter, View, Window, Help -- in that order.
Submenus SHALL follow the documented CS6 ordering. Commands without a handler
SHALL appear disabled rather than be omitted.

#### Scenario: All top-level menus are present
- **WHEN** the frame is shown
- **THEN** the menu bar contains File, Edit, Image, Layer, Type, Select, Filter, View, Window, and Help in that order

#### Scenario: Unimplemented leaf is disabled
- **WHEN** a documented command has no registered handler
- **THEN** its menu item is visible and disabled

### Requirement: Dispatch by command identifier
The system SHALL dispatch a triggered command to a handler registered against
that command's identifier. Triggering a command with no registered handler SHALL
be a no-op that does not crash.

#### Scenario: Registered handler runs
- **WHEN** a menu item for a command with a registered handler is triggered
- **THEN** the handler runs and performs the command's action

#### Scenario: Unregistered command is inert
- **WHEN** a command with no registered handler is triggered
- **THEN** the application does not report an error and remains usable

### Requirement: Enablement recomputed on menu open
The system SHALL evaluate each command's enablement predicate when its menu is
opened, so document-dependent commands are greyed rather than hidden when they do
not apply, and become enabled without rebuilding the menu.

#### Scenario: No document open
- **WHEN** no document is loaded and a document-requiring menu is opened
- **THEN** the document-requiring commands are disabled

#### Scenario: Document becomes available
- **WHEN** a document is loaded and the same menu is reopened
- **THEN** the document-requiring commands are enabled

### Requirement: Dynamic command labels
The system SHALL update labels whose meaning depends on state without rebuilding
the menu; in particular the undo command SHALL read "Undo" or "Redo" according to
the current history state.

#### Scenario: Undo label tracks history
- **WHEN** the document has no undoable step
- **THEN** the edit-history menu item reflects that no undo is available

#### Scenario: Label updates after an edit
- **WHEN** a document edit is recorded and the Edit menu is reopened
- **THEN** the history command label reflects the new state

### Requirement: Layer management commands in the command table

The system SHALL declare the layer-management commands in the declarative
command table under stable `layer.*` identifiers and SHALL register a handler
for each implemented command. The declared set SHALL include
`layer.merge.layers` (Merge Down / Merge Layers), `layer.merge.visible`,
`layer.merge.clippingMask`, `layer.flatten.image`,
`layer.new.layerFromBackground`, `layer.new.backgroundFromLayer`,
`layer.new.layerViaCopy`, `layer.new.layerViaCut`, `layer.new.groupFromLayers`,
`layer.delete.hiddenLayers`, `layer.select.similar`, `layer.select.linked`,
`layer.link.layers`, `layer.unlink.layers`, `layer.hide.layers`,
`layer.rasterize.fillContent`, `layer.rasterize.layer`, and
`layer.rasterize.allLayers`. Each command that does not apply to the current
selection or document SHALL be disabled by an enable provider evaluated when the
menu is opened. The `Type`, `Shape`, `Vector Mask`, `Smart Object`, `Video`, and
`3D` rasterize entries SHALL remain in the table with no handler and SHALL stay
visible and disabled. Changing a label or shortcut SHALL NOT change an
identifier.

#### Scenario: A registered management command dispatches

- **WHEN** a management menu item with a registered handler is triggered
- **THEN** its handler runs and performs the command's action

#### Scenario: Inapplicable commands are disabled on open

- **WHEN** a document or selection makes a management command inapplicable and
  the menu is opened
- **THEN** that command is greyed rather than hidden, and it becomes enabled
  without rebuilding the menu when it later applies

#### Scenario: A kind-less rasterize variant is present but disabled

- **WHEN** the Rasterize submenu is built
- **THEN** Type, Shape, Vector Mask, Smart Object, Video, and 3D appear with no
  handler and are disabled

### Requirement: Free Transform command in the command table

The system SHALL declare the `Edit > Free Transform` command in the declarative
command table under the stable identifier `edit.freeTransform`, with the label
`Free Transform` and the default shortcut `Ctrl+T`, and SHALL register a handler
for it. The command's enablement predicate SHALL be true only when a document is
open and the current layer is a transformable target as defined by
`layer_can_free_transform` (a raster layer, or a smart-object layer whose
embedded source can be materialized), and false for a group, an adjustment
layer, a Background layer, a position-locked layer, or no layer. Triggering it
SHALL begin a Free Transform session on the current layer and leave the document
unchanged until the session commits. The remaining `Edit > Transform > …` menu
leaves SHALL stay in the table with no handler and remain visible and disabled.
Changing the label or shortcut SHALL NOT change the identifier.

#### Scenario: The command is enabled for a transformable layer

- **WHEN** a raster layer is the current layer and the Edit menu is opened
- **THEN** `Edit > Free Transform` is enabled and shows `Ctrl+T`

#### Scenario: The command is disabled for a non-transformable layer

- **WHEN** the current layer is a group, an adjustment layer, a Background layer, or a position-locked layer, or no document is open
- **THEN** `Edit > Free Transform` is disabled

#### Scenario: Triggering begins a session

- **WHEN** `edit.freeTransform` is triggered on a transformable layer
- **THEN** a Free Transform session begins on that layer and the document and history are unchanged until commit

#### Scenario: The other Transform leaves stay placeholders

- **WHEN** the `Edit > Transform` submenu is opened
- **THEN** `Scale`, `Rotate`, `Skew`, `Distort`, `Perspective`, `Warp`, and `Again` appear with no handler and are disabled

### Requirement: Free Pan command in the command table

The system SHALL declare the `View > Free Pan` command in the declarative
command table under the stable identifier `view.freePan`, with the label
`Free Pan`, no default shortcut, and a checked state that defaults to checked
(free pan on). The command SHALL be checkable. Triggering it SHALL flip the
window's free-pan flag, apply the new state to every open document's canvas
immediately, and persist the state to the session store; the command SHALL never
alter document pixels or record a history state. The command's checked provider
SHALL report the current free-pan flag so the checkmark tracks the state.
Changing the label or shortcut SHALL NOT change the identifier.

#### Scenario: The command is present and checked by default [lct_free_pan_default]

- **WHEN** the View menu is opened with a default session
- **THEN** `View > Free Pan` is present, checkable, and checked

#### Scenario: Triggering applies to every open canvas [lct_free_pan_apply]

- **WHEN** `view.freePan` is triggered while several documents are open
- **THEN** every document's canvas adopts the new free-pan state and no document
  pixels or history state change

#### Scenario: The toggle persists [lct_free_pan_persist]

- **WHEN** Free Pan is toggled and the session is saved and reloaded
- **THEN** the checked state is restored on the next launch

