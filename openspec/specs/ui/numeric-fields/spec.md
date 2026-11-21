# numeric-fields Specification

## Purpose
The shared numeric field control with label scrubbing, a slider popup, keyboard handling, and options-bar configuration.
## Requirements
### Requirement: Shared numeric field control

The system SHALL provide one reusable numeric input control (`NumericField`) that
presents a text value, an optional leading text label, and an optional slider
popup, and SHALL be configured by minimum, maximum, step, decimal places, suffix,
integer-or-double value type, and whether the popup is offered. A suffix SHALL be
rendered immediately after the value with no leading or trailing whitespace, so a
`px` suffix renders as `12px` and a percent suffix as `50%`. The control SHALL
emit a preview while a scrub or slider drag is in progress and a single commit
when the edit finishes, and SHALL emit nothing for a programmatic value sync. The
Layers panel Opacity and Fill fields SHALL be a thin configuration of this control
(`0..=100`, integer, `%` suffix) rather than a separate implementation, and every
control migrated to it SHALL keep its existing stored-value conversion.

#### Scenario: A configured field enforces its range and formatting [lpn_config]

- **WHEN** a `NumericField` is configured with a minimum, maximum, and decimal
  places and the user types an out-of-range value
- **THEN** the committed value is clamped to the range and displayed with the
  configured decimal places and suffix

#### Scenario: A suffix renders with no extra space [lpn_suffix_tight]

- **WHEN** a `NumericField` is configured with a `px` suffix and the value is `12`
- **THEN** the field displays `12px` with no space between the number and the
  suffix

#### Scenario: PercentField is one configuration [lpn_percent_thin]

- **WHEN** the Layers panel Opacity field is shown and edited
- **THEN** it exhibits the shared control's scrubbing, popup, and conversion
  behavior, and no second numeric implementation exists for it

#### Scenario: A programmatic sync emits no edit [lpn_sync]

- **WHEN** the owning panel reflects an existing value into the control without
  user input
- **THEN** no preview or commit is emitted

### Requirement: Label and field scrubbing

Pressing and dragging horizontally on a `NumericField`'s label or its value SHALL
scrub the value by one step per pixel with a short press dead zone and a
horizontal-resize cursor, and releasing SHALL commit the edit as exactly one undo
step. A modifier SHALL scale the scrub sensitivity: `Shift` for a coarse step and
`Ctrl` for a fine step. The whole press-drag-release SHALL preview live and add
no history state until release.

#### Scenario: Dragging the label scrubs the value [lpn_scrub]

- **WHEN** the user presses a `NumericField`'s label and drags horizontally past
  the dead zone
- **THEN** the value changes by the dragged step count, previews live, and one
  undo step is added on release

#### Scenario: A modifier scales the scrub sensitivity [lpn_scrub_mod]

- **WHEN** the same horizontal drag is made with `Shift` held and then with `Ctrl`
  held
- **THEN** the `Shift` drag changes the value by a coarser amount than the plain
  drag and the `Ctrl` drag by a finer amount

#### Scenario: A drag adds one undo state only at release [lpn_scrub_commit]

- **WHEN** a scrub previews several values and is then released
- **THEN** no undo state is added during the drag and exactly one is added on
  release

### Requirement: Slider popup

A `NumericField` configured with a popup SHALL expose a control that opens a
popup hosting the shared tracking slider, and the popup SHALL position itself
relative to the field. Pressing on the slider groove and dragging SHALL track the
value continuously rather than page-stepping or jumping, and the drag SHALL
preview live and commit once on release.

#### Scenario: The popup slider tracks a press-drag [lpn_popup_track]

- **WHEN** the user presses the popup slider groove and drags without releasing
- **THEN** the value follows the pointer continuously, the canvas or target
  updates live, and one undo step is added on release

#### Scenario: The popup opens relative to its field [lpn_popup_place]

- **WHEN** the user opens a `NumericField`'s slider popup
- **THEN** the popup is positioned relative to the field rather than at an
  unrelated location

### Requirement: Popup keyboard handling

While a `NumericField`'s slider popup is open, `Left` and `Right` SHALL change
the value by one step, `Home` and `End` SHALL move it to the minimum and maximum,
and `PageUp` and `PageDown` SHALL move it by one page, regardless of which widget
in the popup holds focus. The key handling SHALL work even when the popup window
never acquires keyboard focus.

#### Scenario: Arrow keys step the popup value [lpn_keys_arrows]

- **WHEN** the popup is open and the user presses `Right` and then `Left`
- **THEN** the value increases by one step and then decreases by one step, and
  each change previews and commits like a drag

#### Scenario: Home, End, and Page keys navigate the range [lpn_keys_nav]

- **WHEN** the popup is open and the user presses `End`, `Home`, `PageUp`, and
  `PageDown`
- **THEN** the value moves to the maximum, the minimum, and by one page in each
  direction respectively

### Requirement: Options-bar numeric field configuration

The options-bar paint controls SHALL configure their units and popups correctly:
the Hardness, Opacity, and Flow controls SHALL display a `%` suffix, and the
Feather control SHALL be configured without a slider popup so it opens no popup
when clicked. The new-document and layer unit controls SHALL use a `px` suffix
with no leading space.

#### Scenario: Paint controls show a percent sign [lpn_paint_percent]

- **WHEN** the Brush options bar is shown
- **THEN** the Hardness, Opacity, and Flow fields each display their value with a
  `%` suffix

#### Scenario: Feather opens no popup [lpn_feather_no_popup]

- **WHEN** a selection tool's Feather control is clicked or activated
- **THEN** no slider popup opens and the value is edited only through the field
  itself

#### Scenario: The pixel suffix has no leading space [lpn_px_tight]

- **WHEN** a pixel-dimension field is shown with the value `12`
- **THEN** it displays `12px` with no space before the suffix

### Requirement: Numeric field input alignment

The text of every numeric field SHALL be left-aligned, with the value and its
suffix starting at the field's left edge rather than right-aligned.

#### Scenario: Numeric values are left-aligned [lpn_left_align]

- **WHEN** any numeric field (a layers Opacity/Fill field, an options-bar field,
  or a dialog field) shows a value
- **THEN** the value and its suffix are left-aligned within the field

### Requirement: Shared fields use the system numeric locale

Shared numeric fields SHALL use the same system numeric locale to validate,
parse, and display values, independent of the interface language. Decimal and
fractional edits SHALL accept the locale's decimal separator. Fields SHALL omit
and reject digit grouping, keep their configured precision and tight suffixes,
and preserve their existing clamping, stepping, preview, commit, and invalid
input handling. The result delivered to the model SHALL be numeric. Editing,
saving, and reopening a document SHALL preserve the canonical numeric value
through the existing document storage format.

#### Scenario: Comma decimal editing preserves a fraction [loc_numeric_edit]

- **WHEN** a decimal-capable or fractional shared field is edited under a
  comma-decimal locale by typing `1,5` and committing
- **THEN** it accepts the input, commits numeric 1.5 once, and displays the
  value with a comma and its configured precision and suffix

#### Scenario: Stepping, clamping, and reverting use the same locale [loc_numeric_keys]

- **WHEN** a user types a comma decimal, steps with an arrow key, and then
  tries an out-of-range value and invalid or grouped input
- **THEN** stepping starts at the typed value, the out-of-range commit
  clamps as before, and invalid input uses the existing revert behavior

#### Scenario: A localized document value survives a save and reopen [loc_numeric_roundtrip]

- **WHEN** the user enters a width of `1,5` inches for a document at 72
  pixels per inch, then saves and reopens a PSD
- **THEN** the reopened document is 108 pixels wide at 72 pixels per inch,
  and presenting that width in inches in the same locale uses a comma decimal
