# Spec Delta

## MODIFIED Requirements

### Requirement: Opacity and Fill percent controls

The Layers panel SHALL present Opacity and Fill as percentages in the range
`0..=100`, each through one reusable control that offers a text value, a popup
slider, and scrubbing by pressing and dragging horizontally on the label or the
field. Each field's `%` sign SHALL be rendered inside the value box rather than
beside it. The control SHALL convert between the displayed percentage and the
stored `0..=255` value with `round(pct * 255 / 100)` on edit and `round(value *
100 / 255)` on display, so `100`% is `255`, `50`% is `128`, and `0`% is `0`. The
control SHALL distinguish a live preview from a commit: while a scrub or slider
drag is in progress the control SHALL emit preview changes without adding a
history state, and when the edit finishes it SHALL emit one commit. The panel
SHALL coalesce applying those previews to at most one canvas update per short coalescing interval (about
120 ms), using the latest value, including while the pointer is held stationary
mid-drag. The interval SHALL start on the first change of a gesture and SHALL NOT
be restarted by later changes. A pending preview SHALL NOT be overwritten or discarded by a panel
sync: the field SHALL keep the user's latest value until the preview is applied
on its interval, and a panel refresh MUST NOT apply the pending preview early.
A gesture that returns to its starting value before the interval elapses SHALL
composite nothing and add no state when the final stored byte equals the
starting stored byte. A real view change, a document replacement in the same
view, or a change to the selected targets SHALL cancel the gesture and the
field's pending commit, whether the change happens before or after the preview
fires, and MUST NOT apply a value to the new target. A panel model reset SHALL
NOT by itself cancel the gesture: a transiently empty selection is not a target
change, but a restored non-empty selection whose layers differ SHALL cancel. The
commit SHALL be a no-op when the captured target no longer resolves. A canceled
gesture whose preview already changed pixels SHALL leave those pixels in the
document and mark it modified; it adds no history state. The commit SHALL stop
and discard any pending preview and apply the final value exactly once, adding
exactly one undo state for the whole edit. A completed text edit SHALL also add
one state. The control SHALL emit nothing for a programmatic sync, so a
programmatic sync does not feed back. The existing enable/refusal rules SHALL be
unchanged: Fill is unavailable for a group, and the Background and a fully
locked layer disable both controls. Synthetic display-only rows SHALL NOT count
as eligible targets.

#### Scenario: A percentage maps to the stored byte [lpc_percent]

- **WHEN** Opacity is set to 50 % on an eligible layer
- **THEN** the layer's stored opacity becomes 128 and the document is modified

#### Scenario: A stored byte displays as a percentage [lpc_percent]

- **WHEN** a layer with stored opacity 128 becomes the current row
- **THEN** the Opacity control displays 50 %

#### Scenario: The popup slider and label drag both change the value [lpc_percent]

- **WHEN** the user drags the Opacity popup slider or drags horizontally on the
  Opacity field
- **THEN** the percentage changes, previews are emitted, and the whole drag is
  applied in one undo step

#### Scenario: A drag adds one undo state only at release [lpc_preview]

- **WHEN** an Opacity or Fill drag previews several values and is then released
- **THEN** no undo state was added during the drag and exactly one undo state is
  added on release

#### Scenario: Rapid ticks coalesce into one preview [lpc_coalesced]

- **WHEN** an Opacity or Fill drag emits several values within one coalescing
  interval
- **THEN** at most one preview is applied for that interval, using the latest
  value, no undo state is added, and the interval is not restarted by later
  changes

#### Scenario: A mid-drag pause previews while the button is held [lpc_pause]

- **WHEN** the pointer stops moving during a drag but the button is still held
- **THEN** the latest value is previewed within the coalescing interval and no
  undo state is added

#### Scenario: A fast round trip with no stored-byte change does nothing [lpc_round_trip]

- **WHEN** a drag changes the value and returns to the starting value before the
  interval elapses, the final stored byte equals the starting stored byte, and
  the edit is released
- **THEN** no composite runs and no undo state is added

#### Scenario: The commit wins over a pending preview [lpc_commit_wins]

- **WHEN** the edit finishes while a preview is still pending
- **THEN** the commit applies the final value exactly once and adds exactly one
  undo state

#### Scenario: A panel sync preserves a pending preview [lpc_preserve]

- **WHEN** a panel refresh or model sync runs while a preview is pending
- **THEN** the field keeps the latest user value, no preview is applied early,
  and the pending preview is applied on its coalescing interval

#### Scenario: A target change cancels the gesture [lpc_target_change]

- **WHEN** the view changes, the document is replaced in the same view, or the
  selected targets change, before or after a preview fires
- **THEN** the gesture and the field's pending commit are canceled, no value is
  applied to the new target, and a preview that already ran leaves the document
  modified and dirty with no added history state

#### Scenario: The percent sign is inside the value box [lpr_percent]

- **WHEN** the Opacity or Fill field is shown
- **THEN** its `%` sign is drawn inside the value box rather than beside it

#### Scenario: A programmatic sync does not re-apply the edit [lpc_percent]

- **WHEN** the panel reflects an existing value into the control without user input
- **THEN** no edit is sent and no history state is added

## ADDED Requirements

### Requirement: Disabled opacity and fill show the default

When no layer is selected, or no selected layer is eligible for a field, the
Layers panel SHALL display `100%` in that field and SHALL leave it disabled. The
eligibility test SHALL be over the whole selection: a field with at least one
eligible selected layer remains enabled and SHALL display the current row's
value. This covers an empty selection, the Background, a fully locked layer, a
group's Fill when no selected layer is eligible for Fill, and a synthetic Smart
Filters row. Synthetic display-only rows SHALL NOT count as eligible targets.
The Background's "neither opacity nor fill" means it offers no editable value;
the disabled control still displays the default.

#### Scenario: No selection shows the default [lpc_no_selection]

- **WHEN** the Layers panel has no selected layer
- **THEN** the Opacity and Fill fields both show `100%` and are disabled

#### Scenario: An ineligible target shows the default [lpc_ineligible_default]

- **WHEN** the Background, a fully locked layer, or a synthetic Smart Filters row
  is selected, or a group is the only selected layer and the field is Fill
- **THEN** each field without an eligible target shows `100%` and is disabled

#### Scenario: One eligible layer keeps a multi-selection field enabled [lpc_multi_eligible]

- **WHEN** a selection includes at least one layer eligible for the field
  alongside an ineligible layer
- **THEN** the field stays enabled and displays the current row's value
