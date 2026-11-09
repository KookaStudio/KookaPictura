## ADDED Requirements

### Requirement: Fade compositor

The system SHALL provide `pictura_filters::fade::Fade { opacity: u8, mode: BlendMode }`
and `pictura_filters::fade::fade_layer(original: &Layer, effected: &Layer, fade: &Fade) -> Layer`.
The compositor SHALL return a layer at the union of the two input rectangles,
keeping the effected layer's metadata and non-color channels. Each color channel
and the transparency plane SHALL follow the W3C source-over composite with the
selected blend mode and opacity:

```text
αs' = αs · opacity
αo  = αs' + αb · (1 - αs')
Co  = (1 - αs')·αb·Cb + αs'·((1 - αb)·Cs + αb·B(Cb, Cs))
```

where `Cb`/`αb` come from `original`, `Cs`/`αs` from `effected`, and `B` is the
27-mode W3C blend function. Opacity 0 SHALL reproduce the original pixels inside
the original rectangle and transparency elsewhere, bit-exactly. Opacity 100
with `Normal` SHALL return the effected layer bit-exactly. A missing alpha plane
SHALL be treated as fully opaque. The result SHALL be deterministic and free of
NaN for every finite input.

#### Scenario: Opacity zero restores the original

- **WHEN** `fade_layer` is called with opacity 0 on two layers with different pixels
- **THEN** the result's color and alpha planes equal the original layer inside its rectangle and are transparent outside it

#### Scenario: Opacity one hundred Normal returns the effect

- **WHEN** `fade_layer` is called with opacity 100 and mode `Normal`
- **THEN** the result is bit-identical to the effected layer

#### Scenario: Normal mode is a linear interpolation

- **WHEN** an opaque pixel changes from `(20, 40, 60)` to `(220, 140, 100)` and Fade is applied at 50 % Normal
- **THEN** the result is `(120, 90, 80)` within one level per channel

#### Scenario: A blend mode uses its W3C function

- **WHEN** an opaque pixel changes from `(64, 64, 64)` to `(192, 192, 192)` and Fade is applied at 100 % Multiply
- **THEN** the result is `(48, 48, 48)` within one level per channel

#### Scenario: Alpha composites source-over

- **WHEN** the original pixel is transparent and the effected pixel is opaque at 50 % Normal
- **THEN** the result is half-transparent and keeps the effected color

#### Scenario: Non-color channels come from the effect

- **WHEN** the original and effected layers carry different layer-mask planes
- **THEN** the result's mask plane equals the effected layer's mask plane

#### Scenario: The result covers both rectangles

- **WHEN** the effect grows the layer so its rectangle extends past the original
- **THEN** the result's rectangle is the union and the added pixels follow the opacity rule

### Requirement: Fade availability and last-effect capture

After a destructive filter, Image adjustment, brush or pencil stroke, eraser
stroke, or Magic Eraser commits on a normal pixel layer, the document session
SHALL retain the pre-effect layer, the post-effect layer, the last committed
fade, the current opacity, the current mode, the target layer path, and the
effect name. `Edit ▸ Fade…` SHALL be enabled only while that record exists, the
document is open, and no other edit session is active. Any later pixel,
structural, undo, redo, or history-restore operation SHALL discard the record.
A Smart Filter, a layer-mask target, or a Quick Mask target SHALL NOT create a
record. An effect that leaves the layer unchanged SHALL NOT create a record.

#### Scenario: A filter enables Fade

- **WHEN** Gaussian Blur is committed on a pixel layer
- **THEN** `Edit ▸ Fade…` is enabled and its label names Gaussian Blur

#### Scenario: Fade starts disabled

- **WHEN** a document has just been opened and no effect has been committed
- **THEN** `Edit ▸ Fade…` is disabled

#### Scenario: Another edit disables Fade

- **WHEN** a filter has been committed and then a brush stroke, fill, transform, or another filter commits
- **THEN** `Edit ▸ Fade…` is disabled until that new effect's own record is installed

#### Scenario: Undo disables Fade

- **WHEN** a filter has been committed and the user undoes it
- **THEN** `Edit ▸ Fade…` is disabled

#### Scenario: An adjustment enables Fade

- **WHEN** Invert or a dialog adjustment is committed on a pixel layer
- **THEN** `Edit ▸ Fade…` is enabled and names that adjustment

#### Scenario: A paint or erase stroke enables Fade

- **WHEN** a brush, pencil, eraser, or Magic Eraser edit commits on a pixel layer
- **THEN** `Edit ▸ Fade…` is enabled and names the tool

#### Scenario: Mask and Smart Filter effects do not enable Fade

- **WHEN** a filter is committed with a layer mask as the target, in Quick Mask mode, or as a Smart Filter
- **THEN** `Edit ▸ Fade…` remains disabled

### Requirement: Fade dialog and commit

`Edit ▸ Fade…` SHALL open a dialog with an Opacity control from 0 to 100
(default 100), a Mode menu holding the 27 layer modes (default `Normal`), a
`Preview` checkbox (on by default), and OK and Cancel. The dialog SHALL prefill
the retained opacity and mode. While Preview is on, a control change SHALL
update the canvas without adding a history state; turning Preview off SHALL
restore the last committed fade. Cancel SHALL restore the pre-dialog pixels
bit-identically. OK SHALL add exactly one history state labelled
`Fade <effect>` and SHALL update the retained record so that reopening the
dialog prefills the new opacity and mode and recomputes from the original effect
output rather than compounding. On a Lab document, the modes `Color Dodge`,
`Color Burn`, `Lighten`, `Darken`, `Difference`, and `Exclusion` SHALL be shown
but disabled.

#### Scenario: The dialog opens on the retained values

- **WHEN** Fade is opened after a 40 % Multiply fade
- **THEN** Opacity reads 40, Mode reads Multiply, and Preview is checked

#### Scenario: Preview updates the canvas without history

- **WHEN** the user drags Opacity while Preview is on
- **THEN** the canvas changes and no new history state is added

#### Scenario: Cancel restores the pre-dialog pixels

- **WHEN** the user changes Opacity and then cancels
- **THEN** the document is bit-identical to its state before the dialog opened and history is unchanged

#### Scenario: OK commits one Fade state

- **WHEN** the user accepts the dialog
- **THEN** exactly one history state labelled `Fade <effect>` is added and the canvas holds the faded result

#### Scenario: Reopening recomputes from the effect output

- **WHEN** Fade is applied at 50 % Normal and then reopened and applied at 25 % Normal
- **THEN** the result is 25 % of the original effect output, not 12.5 %

#### Scenario: Lab disables the restricted modes

- **WHEN** the dialog opens on a Lab document
- **THEN** Color Dodge, Color Burn, Lighten, Darken, Difference, and Exclusion are visible but cannot be selected

### Requirement: Fade command wiring

`Edit ▸ Fade…` SHALL be an implemented command in the catalogue, SHALL carry the
`Shift+Ctrl+F` shortcut, and SHALL be dispatched through the command registry.
Its enabled state and label SHALL follow the retained last-effect record.

#### Scenario: The command is implemented and dispatchable

- **WHEN** the command registry describes `edit.fade`
- **THEN** it reports the command as implemented, and dispatching it after an effect opens the Fade dialog
