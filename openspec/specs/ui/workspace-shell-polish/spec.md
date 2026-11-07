# workspace-shell-polish Specification

## Purpose

Cross-cutting surface polish shared by the shell chrome, the Tools panel, and
the widget panels: one idle-less button style, consistent separator weights,
one-step background shades, and DPI-aware tool-slot metrics.

## Requirements

### Requirement: Idle-less global button style

Every tool-panel button and icon (auto-raise) button SHALL render with no idle
outline and no idle background, so at rest it matches the surface that contains
it. The style SHALL apply globally to tool-panel buttons and icon buttons,
including the Layers panel action icons, rather than per site. On hover a button
SHALL show a slightly darker background and a slightly lighter outline; while
pressed it SHALL show a background one shade step darker than hover with the
same lighter outline.

#### Scenario: An idle button matches its surface [lwsp_button_idle]

- **WHEN** a tool-panel or icon button is shown at rest
- **THEN** it draws no outline and no background distinct from its containing
  surface

#### Scenario: Hover and pressed differ by one step [lwsp_button_states]

- **WHEN** a button is hovered and then pressed
- **THEN** hover shows a slightly darker background with a lighter outline, and
  pressed shows a one-step-darker background with the same lighter outline

#### Scenario: The style reaches the Layers icons [lwsp_button_global]

- **WHEN** the Layers panel action icons are shown at rest
- **THEN** they use the same idle-less style as the tool-panel buttons

### Requirement: Chrome and content separator weights

Shell chrome separators SHALL be 2 px and separators drawn inside a widget body
or content SHALL remain 1 px, with both weights declared once in the theme. The
2 px chrome set SHALL cover the menu bar's bottom edge, the options bar's bottom
edge, the status bar's top edge, the icon-group grip, `QMenu::separator`, and
`QToolBar::separator`. The widget-column header line and last bottom line SHALL
be 3 px as specified by `ui/panel-column`. The 1 px set SHALL keep the Layers
filter bar's top and bottom edges, the Info readout grid's inner cross, and the
Layers eye-gutter separator at 1 px, since all three are drawn in-body rather
than as shell chrome.

#### Scenario: Chrome lines are 2 px [lwsp_chrome_2px]

- **WHEN** the menu bar, options bar, status bar, icon-group grip, a `QMenu`
  separator, and a `QToolBar` separator are measured
- **THEN** each separator is 2 px from the shared chrome width constant

#### Scenario: In-body lines stay 1 px [lwsp_body_1px]

- **WHEN** the Layers filter bar edges, the Info readout grid's inner cross, and
  the Layers eye-gutter separator are measured
- **THEN** each is 1 px

### Requirement: Explicit CS6 surface palette

The theme SHALL declare the dark surface palette as explicit colours at the
default brightness level, not as derived shade steps: the panel, widget-column,
toolbar, menu-bar, active file-tab, and Layers-panel surfaces SHALL be `#4d4d4d`;
the in-body 1 px separator lines and the table/list backgrounds SHALL be
`#404040`; the column-header, panel-group-header, and file-bar backgrounds SHALL
be `#363636`; the outer 1 px and 2 px shell borders and the active button
background SHALL be `#2e2e2e`; the workspace background SHALL be `#1f1f1f`; the
input backgrounds and text-button backgrounds at rest and on hover SHALL be
`#3b3b3b`; the hovered and active button outlines and the input outlines SHALL be
`#595959`; and the pressed text-button background SHALL be `#303030`. Every
colour SHALL be declared once in the theme, and no default-level surface SHALL be
computed from a shade step.

#### Scenario: Surfaces use the palette colours [lwsp_palette_surfaces]

- **WHEN** the panel, widget column, toolbar, menu bar, active file tab, Layers
  panel, table background, the column and group headers, the file bar, and the
  workspace are shown at the default brightness level
- **THEN** each uses its declared palette colour and the panel family is `#4d4d4d`

#### Scenario: Inner and outer lines differ [lwsp_palette_lines]

- **WHEN** an in-body 1 px separator line and an outer shell border are shown
- **THEN** the in-body line is `#404040` and the outer border is `#2e2e2e`

#### Scenario: Text button states [lwsp_palette_button]

- **WHEN** a text button is shown at rest, hovered, and pressed
- **THEN** its background is `#3b3b3b` at rest and on hover, its outline is
  `#595959`, and its pressed background is `#303030`

### Requirement: Brightness levels shift the palette

The four brightness levels SHALL keep the default level's exact palette and SHALL
shift every palette colour by a uniform lightness step for the other levels, so
the brightness preference changes the whole shell consistently while the
default-level values remain exactly as declared. Changing the level SHALL
re-apply the palette and invalidate the icon caches.

#### Scenario: The default level is exact [lwsp_level_default]

- **WHEN** the brightness level is the default
- **THEN** every surface uses the exact palette colour declared above

#### Scenario: Other levels shift uniformly [lwsp_level_shift]

- **WHEN** the brightness level is changed away from the default
- **THEN** every surface shifts by the same lightness step and the shell stays
  internally consistent

### Requirement: DPI-aware tool-slot metrics

The tool-slot metrics SHALL be defined once at the 96-DPI base: a 36×28
footprint shared by every slot, with a maximum icon size of 24×20. The idle body
of a slot that draws no outline SHALL be the footprint minus twice the shared
1 px panel border (34×26 at 96 DPI), derived from the footprint and the border
constant rather than declared as separate constants. The metrics SHALL scale
with the DPI of the screen the Tools panel is on, and the scaling SHALL guard
against Qt 6's own high-DPI scaling so a slot is not scaled twice. The chosen
mechanism SHALL be recorded in the change design.

#### Scenario: Base metrics at 96 DPI [lwsp_slot_base]

- **WHEN** the Tools panel is shown on a 96-DPI screen
- **THEN** an outlined slot has a 36×28 footprint, an unoutlined slot's body is
  34×26 derived from the footprint minus the border, and an icon is at most 24×20

#### Scenario: Metrics scale on a high-DPI screen [lwsp_slot_scaled]

- **WHEN** the Tools panel is shown on a high-DPI screen
- **THEN** the slot and icon metrics scale from the one base definition and are
  not scaled twice

### Requirement: Paint Mask Mode toolbar button

The Tools panel SHALL show a working Paint Mask Mode toggle button in the slot
immediately to the left of the screen-mode button, retaining the existing
one-column stacked arrangement and shared slot metrics. Its checked state SHALL
reflect the active document's Quick Mask mode. A click or `Q` SHALL toggle that
mode through the same command, and the control SHALL be disabled without a
document. Double-click SHALL open Quick Mask Options without unintentionally
entering or leaving the mode; `Alt`-click SHALL change display polarity without
toggling mode. Text entry and modal dialogs SHALL retain their normal ownership
of keyboard input.

#### Scenario: The button sits left of screen mode and toggles [lwsp_paint_mask_stub]
- **WHEN** a document is open and the Paint Mask button is clicked and then `Q` is pressed
- **THEN** the button retains its footer position, Quick Mask enters and leaves, and the button tracks the mode rather than an independent checked state

#### Scenario: The button follows document availability [lwsp_paint_mask_document]
- **WHEN** the active document changes or the last document closes
- **THEN** the button shows the active document's mode and is disabled when no document remains

#### Scenario: Options gestures do not toggle the mode [lwsp_paint_mask_gestures]
- **WHEN** the button is double-clicked or `Alt`-clicked in either Standard or Quick Mask mode
- **THEN** the options dialog opens or polarity changes respectively, without changing whether Quick Mask is active

#### Scenario: Typing Q does not toggle Quick Mask [lwsp_paint_mask_text]
- **WHEN** `Q` is typed into a text editor or an owning modal dialog
- **THEN** normal text or dialog input occurs and Quick Mask mode is unchanged

### Requirement: Central-band workspace frame

The central band — the region below the menu bar and above the status bar,
including the full-width options bar and the panel columns — SHALL draw a 3 px
frame on its left and right edges. Each edge SHALL be 2 px of the panel colour on
the outside and 1 px of the outer-border colour on the inside, so the 1 px line
is always adjacent to the content: on the left edge 2 px then 1 px, on the right
edge 1 px then 2 px. The menu bar and status bar SHALL NOT be framed, and the
frame SHALL NOT overlap or clip any column, canvas, or options-bar content.

#### Scenario: The frame layers 2 px then 1 px [lwsp_frame_layers]

- **WHEN** the left and right edges of the central band are inspected
- **THEN** the left edge is 2 px panel then 1 px border and the right edge is
  1 px border then 2 px panel

#### Scenario: Frame content is not clipped [lwsp_frame_no_clip]

- **WHEN** the frame is shown with the tools, canvas, and widget columns
- **THEN** no column, canvas, or options-bar content is overlapped by the frame
