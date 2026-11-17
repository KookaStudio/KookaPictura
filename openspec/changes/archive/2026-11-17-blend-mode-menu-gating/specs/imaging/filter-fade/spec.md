# Spec Delta

## MODIFIED Requirements

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
output rather than compounding. The menu SHALL use the engine's blend-mode
availability for the document's effective colour mode and bit depth. On a Lab
document, the modes `Color Dodge`, `Color Burn`, `Darken`, `Lighten`,
`Difference`, `Exclusion`, `Subtract`, and `Divide` SHALL be shown but
disabled. On a 32-bit document, the modes `Color Burn`, `Linear Burn`, `Screen`,
`Color Dodge`, `Overlay`, `Soft Light`, `Hard Light`, `Vivid Light`, `Linear
Light`, `Pin Light`, `Hard Mix`, and `Exclusion` SHALL be shown but disabled.
An 8- or 16-bit RGB, Grayscale, or CMYK document SHALL offer all 27 modes. A
Bitmap or Indexed document SHALL follow the same Normal-only availability as
the layer controls.

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
- **THEN** Color Dodge, Color Burn, Darken, Lighten, Difference, Exclusion, Subtract, and Divide are visible but cannot be selected

#### Scenario: 32-bit disables the modes outside the HDR list [fade_32]

- **WHEN** the dialog opens on a 32-bit document
- **THEN** Color Burn, Linear Burn, Screen, Color Dodge, Overlay, Soft Light, Hard Light, Vivid Light, Linear Light, Pin Light, Hard Mix, and Exclusion are visible but cannot be selected, and the 15 documented 32-bit modes are selectable

#### Scenario: An 8-bit RGB document offers all modes

- **WHEN** the dialog opens on an 8-bit RGB document
- **THEN** all 27 modes are selectable
