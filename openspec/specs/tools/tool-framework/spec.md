# tool-framework Specification

## Purpose
The tool registry, active tool, tools panel, options bar, and canvas pointer routing with cursor hints.
## Requirements
### Requirement: Tool registry and active tool

The system SHALL define a fixed set of tools identified by a stable `ToolId`
and SHALL track exactly one active tool. The implemented set SHALL remain the
ten tools (Move, Marquee, Lasso, Quick Selection, Crop, Eyedropper, Brush,
Pencil, Hand, Zoom). The system SHALL refuse to activate a tool that has no
engine. Selecting a tool SHALL be possible from a single-letter keyboard
shortcut, and every tool in one flyout group SHALL share that group's slot
letter. The system SHALL honour the `Use Shift Key For Tool Switch` preference:
when it is on (the default), pressing the letter alone SHALL activate the slot's
current member and `Shift`+letter SHALL cycle to the next implemented member of
the group; when it is off, the letter alone SHALL cycle. Cycling SHALL wrap
deterministically and SHALL skip members that are not implemented. A group with
no implemented member SHALL react to no letter.

#### Scenario: Activating a tool by shortcut [m40_shift]

- **WHEN** the user presses a tool slot's letter
- **THEN** the slot's current member becomes active and no other tool remains active

#### Scenario: Cycling the paint tool slot [m40_shift]

- **WHEN** `Shift+B` is pressed while Brush is active and `Use Shift Key For Tool Switch` is on
- **THEN** Pencil becomes active

#### Scenario: The preference controls whether Shift is required [m40_shift]

- **WHEN** the same letter is pressed with the preference off
- **THEN** the letter alone advances to the next implemented member of the group

#### Scenario: A group with no implemented member does nothing [m40_shift]

- **WHEN** the letter of a group whose members are all unimplemented is pressed
- **THEN** the active tool does not change

### Requirement: Tools panel

The system SHALL present the tools in a tabless, atomic Tools column as a list
of flyout slots in CS6 order, showing the active tool distinctly, and SHALL
expose the panel through the `Window` menu. The Tools column SHALL be hosted like
a `PanelColumn`, with a column header as its top area and the tool grid as a
single plain content child, and SHALL have no tab bar and no `PanelGroup`. The
column SHALL be placeable as a sibling of any widget column on either side of
the document tab area. The Tools column and widget panels SHALL NOT be combined
in either direction: the Tools column SHALL NOT tabify with a widget panel or
group, a widget panel SHALL NOT be inserted into the Tools column, and the Tools
column SHALL NOT be dropped into a widget group, panel, column, or float. The
panel SHALL default to a single column of slots and SHALL be reflowable to two
columns and back to one. A slot whose group has more than one member SHALL show
a filled triangle at the slot's lower-right corner, whether or not those members
are implemented, while keeping the slot icon centred, and SHALL reveal the
group's members when the pointer press is held past the hold delay or the slot is
right-clicked. An unimplemented tool SHALL be shown disabled with the tooltip
`<label> — not implemented yet`. The panel SHALL keep the foreground/background
colour control and the screen-mode control pinned below the slots. The
foreground/background control SHALL provide, besides the two swatches and the
default-colour (X) reset, a swap control drawn as a top-left and a down-left
arrow that exchanges the foreground and background colours, the `X` key SHALL
swap them when no tool shortcut claims it, and the `D` key SHALL reset them to
the default colours (black foreground, white background) when no tool shortcut
claims it. The panel SHALL show a Paint Mask Mode toggle button in the slot
immediately to the left of the screen-mode control, and the screen-mode control
SHALL open the screen-mode menu. A left-button drag on the Tools column's header
SHALL be tracked for the whole gesture, whether the column starts column-hosted,
floating, or a sibling column, and on release SHALL place the Tools column as a
sibling of the widget column under the pointer or between two widget columns.
While the pointer is in the workspace outer left or right band the column SHALL
show the same blue edge indicator the widget columns use, anchored on the
outermost visible column on that side, and the release SHALL commit the Tools
column on that side as a column. A release with no widget column under the
pointer SHALL float the Tools column at the cursor as a frameless `Qt::Tool`
top-level window parented to (transient for) the main window — no title bar, no
window decorations, and no taskbar entry — rather than an in-window overlay, so
a floating column stays floating and a column can be floated by dragging. The
floating Tools column SHALL be movable outside the main window, its movement
clamped to the available geometry of the screen under the target point so it
cannot be lost off-screen. While floating it SHALL keep its fixed content width
and SHALL remain re-draggable, and it SHALL NOT be resizable: it SHALL offer no
resize grip and SHALL take the minimum its tool-grid content needs. The Tools
column SHALL NOT take an iconic rail mode. The implemented tool set and the
active-tool contract SHALL be unchanged.

#### Scenario: Tools panel reflects the active tool [m23_toolbox]

- **WHEN** a tool becomes active by shortcut or by clicking its button
- **THEN** the Tools panel marks that tool's slot as active

#### Scenario: Tools default to a single column of slots [m40_columns]

- **WHEN** the Tools panel is shown with no saved column choice
- **THEN** the tool buttons are arranged in a single column of flyout slots

#### Scenario: The Tools column is tabless and atomic [tpc_atomic]

- **WHEN** the Tools column is shown
- **THEN** it renders a column header and the tool grid with no tab bar and no
  `PanelGroup`, and it is a sibling column of the widget columns

#### Scenario: The footer controls [ltf_footer_controls]

- **WHEN** the Tools panel's footer is shown
- **THEN** the foreground/background swap control draws a top-left and a
  down-left arrow, the Paint Mask Mode toggle sits immediately left of the
  screen-mode control, and the screen-mode control opens the screen-mode menu

#### Scenario: The header drag places the column among columns [tpc_sibling]

- **WHEN** the Tools column's header is dragged to the left or right side of a
  widget column, or between two widget columns
- **THEN** a single blue indicator marks that boundary and on release the Tools
  column is placed as a sibling column at that boundary with its fixed content
  width

#### Scenario: The outer band commits the tools column [tpc_edge_commit]

- **WHEN** the Tools column's header is dragged into the workspace outer left or
  right band
- **THEN** the shared blue edge indicator is shown on the outermost visible
  column on that side and the release commits the Tools column on that side as a
  column

#### Scenario: The floating tools column is a frameless tool window [tpc_inwindow_float]

- **WHEN** the Tools column's header is dragged and released with no widget
  column under the pointer
- **THEN** the Tools column floats at the cursor as a frameless `Qt::Tool`
  top-level window parented to the main window with no title bar, decorations,
  or taskbar entry, movable outside the main window and clamped to the screen

#### Scenario: A widget drag cannot combine with the tools column [tpc_atomic_target]

- **WHEN** a widget panel or group is dragged toward the Tools column, or the
  Tools column is dragged toward a widget panel or group
- **THEN** no tabification, insertion, or combined drop occurs and no insertion
  indicator is shown for the forbidden combination

### Requirement: Options bar

The system SHALL present an options bar whose controls change with the active
tool, and SHALL allow the options bar to be shown or hidden. Each selection tool
SHALL expose its combine mode; Quick Selection SHALL expose a tolerance. Brush
and Pencil SHALL expose size, hardness, opacity, flow, and paint mode; Pencil
SHALL additionally expose Auto Erase. The options bar's numeric controls SHALL
use the shared numeric field control, so each offers a scrubbing label and a
slider popup, and a change to one control SHALL stay in sync with any other
control bound to the same tool value. Every brush-settings control SHALL be
idle-less (no idle outline or background), and the options bar's body SHALL size
to its controls so the bar reserves no vertical space for an idle button and is
not over-tall.

#### Scenario: Options follow the active tool

- **WHEN** the active tool changes
- **THEN** the options bar shows that tool's controls

#### Scenario: Options bar toggle

- **WHEN** the user toggles the options bar
- **THEN** it is shown or hidden without changing the active tool

#### Scenario: A numeric option can be scrubbed and popup-edited [ltf_options_numeric]

- **WHEN** the user interacts with a numeric options-bar control such as the
  brush size or a selection tolerance
- **THEN** it offers a scrubbing label and a slider popup and applies the change
  to the next operation

#### Scenario: The bar is not over-tall [ltf_options_compact]

- **WHEN** the options bar is shown for a tool with a brush-settings control
- **THEN** the brush-settings control is idle-less and the bar's body height
  matches its controls with no reserved idle-button space

### Requirement: Canvas pointer routing to the active tool
The system SHALL route canvas pointer press, move, and release events to the
active tool with image-space coordinates. When the Hand tool is active the canvas
SHALL pan on left-drag as before; other tools SHALL receive the events.

#### Scenario: Hand tool pans
- **WHEN** the Hand tool is active and the user left-drags the canvas
- **THEN** the canvas pans

#### Scenario: A drawing tool receives the drag
- **WHEN** a selection or crop tool is active and the user presses on the canvas
- **THEN** that tool begins its drag and is given the pressed image coordinates

### Requirement: Tool status and cursor hints

The system SHALL show a context hint for the active tool in the bottom status bar
as a hint bar of bordered keycaps with descriptions, and SHALL set a
tool-appropriate cursor while the tool is active. For Brush and Pencil the cursor
SHALL be blank so the drawn brush-size ring is the pointer affordance. When the
active tool would edit pixels and the target layer is pixel-locked, the cursor
SHALL indicate that the action is not allowed, and a refusal SHALL be reported to
the user rather than the action silently doing nothing. **The canvas SHALL also
show a Block/Forbidden cursor when a pixel-editing tool's active layer is
invisible (an edit would be refused), and the transient Alt eyedropper SHALL
show the eyedropper cursor while Alt is held with a paint tool active. The
cursor SHALL be resolved in one place with the precedence transient eyedropper >
transform session > blank paint > invisible target > pixel-locked target >
tool default, so the branches cannot disagree.**

#### Scenario: Status hint reflects the tool

- **WHEN** a tool becomes active
- **THEN** the status-bar hint bar shows that tool's context keycaps

#### Scenario: Brush uses a blank cursor [ltf_blank_cursor]

- **WHEN** Brush or Pencil is active over the canvas
- **THEN** the canvas cursor is blank and the brush-size ring is drawn

#### Scenario: A locked target marks the cursor [ltf_locked_cursor]

- **WHEN** a pixel-editing tool is active over a pixel-locked layer
- **THEN** the canvas cursor indicates the action is not allowed and an attempted
  edit reports a refusal

#### Scenario: An invisible target marks the cursor [ltf_invisible_cursor]

- **WHEN** a pixel-editing tool is active over a layer whose visibility is off
- **THEN** the canvas shows a Block/Forbidden cursor and an attempted paint is
  refused

#### Scenario: The transient eyedropper cursor shows [ltf_alt_cursor]

- **WHEN** Alt is held with a paint tool active over the canvas
- **THEN** the canvas shows the eyedropper cursor, which takes precedence over
  the blank paint cursor

### Requirement: Tools panel column layout

The system SHALL present the Tools column with a column header that carries one
width-toggle control and no separate `Tools` title label. The control SHALL share
the widget column's toggle style and SHALL reflow the slots between one and two
columns, showing the icon of the layout it switches to, using the bundled
`panel.columnsTwo` and `panel.columnsOne` icons. The Tools column SHALL NOT have
an iconic rail mode, so the control SHALL change the tool-grid width and SHALL
NOT collapse the column to an icon rail. In two columns the slots SHALL be laid
out left-to-right and top-to-bottom. The one- and two-column widths SHALL fit the
slot buttons plus the foreground/background control rather than a fixed constant,
so the foreground/background control fits within the current column width and
never widens it. Each width SHALL be the tight content width plus only the body
layout's margins, so the column carries no extra horizontal space in either
mode. Each width SHALL be fixed: the column's minimum and maximum width SHALL
both equal its content width for the active column count, recomputed when the
column count changes and while the column is floating, and dragging the column
separator SHALL NOT resize it. Both the content width and the content height
SHALL be derived from one content formula and SHALL be recomputed on every
column-count change and on every float/column-host change, releasing the stale
fixed axis before re-fixing it, so neither the one- nor the two-column layout is
cut off or left over-tall. While the column is hosted in the workspace the tool
grid SHALL fill its column height; while the column is floating the overlay SHALL
be a frameless `Qt::Tool` top-level window parented to the main window and its
height SHALL be fixed to the minimum its content needs, with no resize grip and
no drag-resize. The chosen column count SHALL persist in the session store and
SHALL load as one column when the store is missing, older, or the value is out of
range. The foreground/background control and the screen-mode control SHALL remain
below the slots in both layouts. A missing column icon SHALL fall back to a text
arrow rather than fail.

#### Scenario: The double-arrow toggles to two columns and back [m41_tools]

- **WHEN** the user clicks the header double arrow while in one column
- **THEN** the 23 slots reflow into two columns and the button shows the
  one-column icon, and a second click restores one column

#### Scenario: The header has no Tools label [m41_tools]

- **WHEN** the Tools column's header is inspected
- **THEN** it carries the double-arrow control and no `Tools` text label

#### Scenario: Each mode sizes exactly to its content [m45_tools_sizing]

- **WHEN** the Tools column is shown in one column and then switched to two
  columns, floating and column-hosted
- **THEN** in each mode the width equals that mode's content width and the
  height equals that mode's content height while floating, with no stale lock, no
  cut-off, and no leftover extra height

#### Scenario: The width fits the content [m41_tools]

- **WHEN** the one- and two-column widths are queried
- **THEN** each is derived from the slot and foreground/background control sizes
  and the foreground/background control fits within the current column width

#### Scenario: The width has no extra horizontal space [m42_tools]

- **WHEN** the column's actual width is compared with the content width in either
  one- or two-column mode
- **THEN** the difference is only the body layout's margins

#### Scenario: The width is fixed [m43_tools]

- **WHEN** the Tools column's minimum and maximum widths are queried in one- or
  two-column mode
- **THEN** both equal the content width for that mode

#### Scenario: The floating height is fixed [m44_toolsfloat]

- **WHEN** the Tools column is floating and a resize is attempted
- **THEN** its height stays the minimum its content needs and does not change

#### Scenario: The tools column has no iconic mode [tpc_no_iconic]

- **WHEN** the Tools column's width control is activated
- **THEN** it switches the tool grid between one and two columns and never
  collapses the column to an icon rail

#### Scenario: Column count persists [m40_session]

- **WHEN** the user selects two columns and the session is saved and reloaded
- **THEN** the panel opens in two columns

### Requirement: Tool flyout indicator and opening

The system SHALL mark every slot whose group has more than one member with a
filled triangle at the lower-right corner, including a group whose members are
all unimplemented, and SHALL NOT mark a single-member slot. Holding the pointer
press on a slot past the hold delay SHALL open the slot's flyout menu positioned
to the right of the button, or to the left when the right side does not fit,
never covering the button that opened it; a right-click SHALL open it
immediately; releasing the press before the delay SHALL activate the slot's
current tool and SHALL NOT open the menu. While a slot's flyout is open, a press
on another slot SHALL close the open flyout and activate that slot (or open its
menu) within the same press, so switching tools takes only one click. Each
flyout item SHALL carry a left margin for its icon and a tighter gap between the
icon and the label. A single-member slot SHALL NOT open a flyout. The system
SHALL expose a deterministic hook that opens a slot's flyout for the self-test.

#### Scenario: A multi-member slot shows the triangle [m40_flyout]

- **WHEN** a slot's group has two or more members
- **THEN** the slot paints a filled triangle at its lower-right corner

#### Scenario: A single-member slot shows no triangle [m40_flyout]

- **WHEN** a slot's group has exactly one member
- **THEN** the slot paints no triangle and opens no flyout

#### Scenario: Holding opens the flyout below the button [m40_flyout]

- **WHEN** the pointer press is held on a multi-member slot past the hold delay
- **THEN** the slot's flyout opens to the right of the button, or to the left
  when the right side does not fit, and does not cover the button

#### Scenario: A right-click opens the flyout immediately [m40_flyout]

- **WHEN** the user right-clicks a multi-member slot
- **THEN** the flyout opens without waiting for the hold delay

#### Scenario: A quick release selects the current tool [m40_flyout]

- **WHEN** the user presses and releases a slot before the hold delay
- **THEN** the slot's current implemented tool becomes active and no flyout opens

#### Scenario: One click switches tools while a flyout is open [ltf_flyout_one_click]

- **WHEN** a slot's flyout is open and the user presses another slot
- **THEN** the open flyout closes and that slot activates (or its own menu opens)
  in the same press, with no second click needed

#### Scenario: Flyout items have icon margin and a tight gap [ltf_flyout_item_gap]

- **WHEN** a flyout menu is shown
- **THEN** each item's icon has a left margin and the gap between the icon and
  the label is tight

### Requirement: Tool flyout shortcut keys

The system SHALL show every flyout item's slot key right-aligned in the item,
including a disabled item, and SHALL keep every unimplemented item disabled with
the tooltip `<label> — not implemented yet`. The system SHALL display the key
through `QAction::setShortcut` with `QAction::setShortcutVisibleInContextMenu`
enabled, scoped to the flyout with `Qt::WidgetWithChildrenShortcut`, and SHALL
NOT register a window-global shortcut for a flyout item, so a key shared by a
group's members — including a disabled member — cannot steal the slot letter from
the toolbox's keyboard handler.

#### Scenario: Every item shows its shared key [m40_keys]

- **WHEN** a group's flyout is opened
- **THEN** every item shows the group's letter right-aligned

#### Scenario: A disabled item stays disabled with its tooltip [m40_keys]

- **WHEN** the flyout contains a tool with no engine
- **THEN** that item is disabled and its tooltip is `<label> — not implemented yet`

#### Scenario: A disabled item's shared key does not steal the letter [m40_keys]

- **WHEN** a group's flyout has been opened and closed and the group's letter is pressed
- **THEN** the toolbox's keyboard handler runs and the disabled item's shortcut does not consume the key

### Requirement: Shift-key tool switching preference

The system SHALL store a `Use Shift Key For Tool Switch` preference that defaults
to on and SHALL gate the `Shift` requirement for cycling a tool group on it. The
preference SHALL persist in the session store at schema version 4, and a store
that is missing the field or older than version 4 SHALL load the default. Adding
the preference SHALL NOT change the load and save round-trip of the other session
fields.

#### Scenario: The preference defaults to on [m40_session]

- **WHEN** the session preference is read with no saved value
- **THEN** `Use Shift Key For Tool Switch` is on

#### Scenario: The preference round-trips [m40_session]

- **WHEN** the preference is changed, the session is saved, and the store is reloaded
- **THEN** the changed value is restored alongside the other session fields

#### Scenario: An older store loads the default [m40_session]

- **WHEN** a schema-3 store with neither new field is loaded
- **THEN** the column count is 1 and the preference is on

### Requirement: Active layer resolution for tool edits

The application SHALL maintain exactly one active layer for the active document
and SHALL resolve it through one shared active-layer resolver used by every
tool-edit entry point (paint, filter, Free Transform, and content move). The
Layers panel SHALL push its selection into the view when the selection changes,
and a non-PSD raster import SHALL make its imported layer active. A tool edit
SHALL be refused, with the document unchanged and a user-visible refusal, when no
layer is active or when more than one layer is selected. No tool-edit entry point
SHALL fall back to the topmost raster layer.

#### Scenario: Exactly one active layer receives the edit [ltf_active_layer]

- **WHEN** a tool edit runs with exactly one layer active
- **THEN** that layer is the edit target and the other layers are unchanged

#### Scenario: The panel pushes its selection [ltf_active_layer_push]

- **WHEN** the user changes the selected layer in the Layers panel
- **THEN** the view's active layer follows the panel selection and the next tool
  edit targets it

#### Scenario: Zero or multiple selection refuses [ltf_active_layer_refuse]

- **WHEN** a tool edit runs with no layer active or with more than one layer
  selected
- **THEN** the edit is refused with a user-visible refusal, the document is
  unchanged, and no history state is added

### Requirement: Active-layer scope of the resolver

The resolver SHALL be the single source of the edit target and SHALL be consulted
by paint, filter, Free Transform, and content move. The resolver SHALL NOT be
used to block structural layer operations, and a position lock SHALL remain the
only reason a move is refused. When the active layer changes, the tool status and
cursor SHALL reflect the new target without a separate per-tool lookup.

#### Scenario: Every tool edit uses the same resolver [ltf_active_layer_shared]

- **WHEN** paint, a filter, Free Transform, and a content move each run with the
  same single active layer
- **THEN** all four target that layer through the same resolver

### Requirement: Transient alt-tool cursor is resolved centrally

The cursor precedence SHALL live in one `refreshCursor` path so the transient
eyedropper, the invisible-layer refusal, and the existing blank/locked cursors
are not re-evaluated in separate handlers. Changing the active layer, its
visibility, or the held modifiers SHALL refresh the cursor through that one path
without a per-tool lookup.

#### Scenario: One path resolves all cursor states [ltf_cursor_precedence]

- **WHEN** the active layer becomes invisible, then visible again, with a paint
  tool active
- **THEN** the canvas cursor changes through the single precedence path each time
  and no other handler re-sets it

### Requirement: Canvas cursor refreshes on an active-layer change

When the active layer's visibility or identity changes, the canvas cursor SHALL
be refreshed without requiring a pointer move, so an invisible active layer shows
the Block/Forbidden cursor (and a visible one restores the tool cursor)
immediately after an eye toggle or a layer selection. The cursor resolution
itself SHALL keep the existing precedence: a transient Alt eyedropper wins, then
the invisible/locked refusal, then the tool cursor.

#### Scenario: Hiding the active layer shows the Block cursor [tf_visible_cursor_refresh]

- **WHEN** the active layer is made invisible while a pixel-editing tool is
  active and the pointer is over the canvas
- **THEN** the canvas cursor becomes the Block/Forbidden cursor without moving
  the pointer

#### Scenario: Showing the active layer restores the tool cursor [tf_visible_cursor_restore]

- **WHEN** the active layer is made visible again
- **THEN** the canvas cursor returns to the active tool's cursor without moving
  the pointer

### Requirement: Tool refusal follows the active layer

The tool refusal SHALL resolve the **active** single layer's lock state and
visibility — the same layer the paint and filter edits target — not the topmost
pixel layer. The brush/pencil blank-cursor affordance, the Block/Forbidden
refusal cursor, and the edit-refusal status message SHALL all follow this. When the active layer is pixel- or position-locked, or is
invisible, the refusal cursor and message SHALL appear even if the topmost pixel
layer is editable; when the topmost layer is locked but a lower layer is active
and editable, the cursor SHALL stay blank (brush) and the edit SHALL proceed. The
transient Alt-eyedropper precedence SHALL be unchanged.

#### Scenario: A locked active layer refuses with the Block cursor [tf_refusal_active_locked]

- **WHEN** the active layer is pixel-locked and is not the topmost pixel layer
- **THEN** the brush shows the Block/Forbidden cursor and refuses the edit

#### Scenario: A locked topmost layer does not block a lower active layer [tf_refusal_topmost_ignored]

- **WHEN** the topmost pixel layer is locked but the active layer is a lower,
  editable layer
- **THEN** the brush cursor stays blank and the edit targets the active layer

#### Scenario: A panel multi-selection prevents edits [tf_refusal_multi]

- **WHEN** the Layers panel has more than one row selected, or no row
- **THEN** tool edits are refused with no history state

### Requirement: Modal tool-session undo and redo

The tool framework SHALL provide a transient undo/redo session owned by the
active tool. While a tool's session holds steps, the global Edit Undo and Redo
commands SHALL act on the tool session before the document history, and SHALL
fall back to the document history when the session is empty or no tool session
is active. Recording, walking, or discarding a session step SHALL refresh the
Edit Undo and Redo command enabled state, so their shortcuts work while the
session is the only history. A tool session SHALL be discarded when the tool is
deactivated, cancelled, or applied. Applying a session SHALL commit its result
to the document history as a single state.

#### Scenario: Tool undo takes precedence [ltf_session_precedence]

- **WHEN** a modal tool holds session steps and Edit Undo is invoked
- **THEN** the tool session steps back one step and the document history is unchanged

#### Scenario: Fallback to document history [ltf_session_fallback]

- **WHEN** Edit Undo is invoked with an empty tool session or no modal tool active
- **THEN** the document history is undone

#### Scenario: The Undo command is enabled by the session [ltf_session_enables_undo]

- **WHEN** a modal tool records its first session step and the document history is empty
- **THEN** the Edit Undo command is enabled and invoking it walks the session

#### Scenario: Cancel discards the session [ltf_session_cancel]

- **WHEN** a modal tool is cancelled
- **THEN** its session is discarded and no session step is reachable

#### Scenario: Apply commits one state [ltf_session_apply_one]

- **WHEN** a modal tool with several session steps is applied
- **THEN** exactly one document history state is added

### Requirement: Crop options-bar controls

The Crop tool's options bar SHALL present, left to right and separated into
groups by vertical rules: the tool icon; an aspect-ratio select whose first and
default entry is `Ratio` (a free custom aspect) and whose other entries include
the CS6 presets plus `New Crop Preset...` and `Delete Crop Preset...`, `W` and
`H` numeric fields, and a swap-aspect button; a `Clear` control that resets the
ratio and a spirit-level button that toggles a straighten line mode; a grid
overlay menu button and a settings (cog) menu button; `Delete Cropped Pixels` and
a disabled `Content-Aware` placeholder; a reset button; and cancel and apply
buttons. Every icon button (swap-aspect, spirit-level, grid, cog, reset, cancel,
apply) SHALL use a Lucide SVG icon, not a text glyph. In ratio mode the `W`/`H`
fields SHALL show the aspect-ratio values and editing them SHALL set a custom
ratio; only the `W x H x Resolution` entry SHALL show the pixel width/height and
a resolution field with a unit. `New Crop Preset...` SHALL add the current ratio
to the select; `Delete Crop Preset...` SHALL remove the selected user preset. The
spirit-level toggle SHALL arm a straighten mode in which a pressed, dragged,
released line sets the crop angle to the line's inclination and then disarms.
Applying a preset or field SHALL keep the crop box and any active straighten
consistent. The cog menu SHALL offer a `Use Classic Mode` toggle (checked by
default); a Classic drag moves the crop box, a Modern drag repositions the
composite behind a fixed box, and the mode also decides the initial state
(Classic boxless, Modern a centered preview box). The crop options (mode, grid
overlay, ratio, and Delete Cropped Pixels) SHALL persist across restarts.

#### Scenario: Crop option buttons use Lucide icons [ltf_crop_icons]

- **WHEN** the Crop options bar is shown
- **THEN** the swap-aspect, spirit-level, grid, cog, reset, cancel, and apply buttons each carry a Lucide SVG icon and no text glyph

#### Scenario: Classic vs Modern drag mode [ltf_crop_mode]

- **WHEN** `Use Classic Mode` is toggled in the cog menu and the active box is dragged from inside
- **THEN** Classic moves the box and Modern repositions the composite behind the fixed box

#### Scenario: The mode sets the initial box [ltf_crop_mode_init]

- **WHEN** the Crop tool is selected with Modern off (Classic) vs on
- **THEN** Classic shows no box until one is drawn, and Modern shows a centered preview box immediately

#### Scenario: The ratio menu and resolution mode [ltf_crop_ratio_menu]

- **WHEN** a ratio is chosen with `Ratio` or a preset vs `W x H x Resolution`
- **THEN** the `W`/`H` fields show the bare aspect-ratio values in ratio mode and the pixel width/height plus a resolution field and unit only in resolution mode

#### Scenario: Crop options persist [ltf_crop_persist]

- **WHEN** a crop option is changed and the app restarts
- **THEN** the mode, grid overlay, ratio, and Delete Cropped Pixels are restored to the changed values

#### Scenario: A preset locks the box ratio [ltf_crop_preset]

- **WHEN** a ratio preset is selected in the Crop options bar
- **THEN** the crop box is fitted to that ratio

#### Scenario: A user preset is added and removed [ltf_crop_preset_crud]

- **WHEN** `New Crop Preset...` is used with a ratio and later `Delete Crop
  Preset...` removes it
- **THEN** the select gains and then loses that entry

#### Scenario: A ratio default is the free Ratio entry [ltf_crop_ratio_default]

- **WHEN** the Crop options bar is shown for the first time or after a reset
- **THEN** the aspect-ratio select shows the `Ratio` entry

#### Scenario: Crop control groups are separated [ltf_crop_separators]

- **WHEN** the Crop options bar is shown
- **THEN** its control groups are separated by vertical rules

#### Scenario: Content-Aware is a disabled placeholder [ltf_crop_content_aware]

- **WHEN** the Crop options bar is shown
- **THEN** a `Content-Aware` checkbox is present and disabled

#### Scenario: W and H set the ratio [ltf_crop_fields]

- **WHEN** the `W` or `H` ratio field is committed
- **THEN** the crop box is fitted to that width:height ratio

#### Scenario: Straighten draws a line [ltf_crop_straighten]

- **WHEN** the spirit-level toggle is armed and a line is dragged on the canvas
- **THEN** the crop angle becomes the line's inclination and the mode disarms

### Requirement: Active tool icon separator

Every active tool's options page SHALL draw a vertical separator immediately after
its tool icon, before its other controls, matching the Crop page.

#### Scenario: Every page separates the tool icon [ltf_tool_icon_separator]

- **WHEN** any tool's options bar is shown
- **THEN** a vertical rule separates the tool icon from the next control

### Requirement: Move options-bar controls

The Move tool's options bar SHALL offer an `Auto-Select` choice of Group or Layer,
a `Show Transform Controls` checkbox, the align and distribute button sets, and a
three-dots menu listing the align and distribute actions plus an `Align To:`
choice of Selection or Canvas. Choosing `Align To: Canvas` SHALL align to the
canvas bounds; `Selection` SHALL align to the active selection.

#### Scenario: Align To Canvas [ltf_move_align_canvas]

- **WHEN** `Align To: Canvas` is chosen and an align action is invoked
- **THEN** the selected layers are aligned to the canvas bounds

#### Scenario: Auto-Select and transform controls are present [ltf_move_controls]

- **WHEN** the Move options bar is shown
- **THEN** the `Auto-Select`, `Show Transform Controls`, align, distribute, and three-dots controls are present

### Requirement: Eyedropper options-bar controls

The Eyedropper tool's options bar SHALL offer a `Sample Size` (Point Sample
through the N by N average sizes up to 101 by 101), a `Sample` scope (Current
Layer, Current & Below, All Layers, All Layers No Adjustments, Current & Below
No Adjustments), and a `Show Sampling Ring` checkbox. A sample SHALL average the
chosen size and read the chosen scope.

#### Scenario: Sample size and scope are honored [ltf_eyedropper_sample]

- **WHEN** a sample size and scope are chosen and a pixel is sampled
- **THEN** the foreground is the average of that size read from that scope

#### Scenario: Sampling ring toggles [ltf_eyedropper_ring]

- **WHEN** `Show Sampling Ring` is checked and the eyedropper hovers the canvas
- **THEN** a sampling ring is drawn around the pointer

