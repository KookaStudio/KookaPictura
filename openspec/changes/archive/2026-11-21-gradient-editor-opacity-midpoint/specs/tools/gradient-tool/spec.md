## ADDED Requirements

### Requirement: Gradient Editor from the Gradient tool

The Gradient tool's options bar SHALL open the Gradient Editor from the main
part of the gradient sample while keeping the preset menu on the sample's arrow.
The editor SHALL open with the tool's current ramp. Accepting the editor SHALL
make the edited solid gradient — including its opacity stops and midpoints — the
ramp the next drag draws and SHALL leave no preset checked; cancelling SHALL keep
the ramp the tool had. Choosing a preset from the sample's menu SHALL replace
the edited ramp with that built-in gradient.

#### Scenario: The sample opens the editor [gt_editor_opens]

- **WHEN** the Gradient tool's gradient sample is clicked
- **THEN** the Gradient Editor opens with the tool's current colour stops

#### Scenario: An accepted gradient is drawn [gt_editor_applies]

- **WHEN** the editor is accepted after its start opacity stop is set to 0 % and a colour midpoint is moved off 50 %
- **THEN** the next drag draws the edited ramp: the start is transparent and the midpoint's even-mix point has moved

#### Scenario: Cancelling keeps the previous ramp [gt_editor_cancels]

- **WHEN** the editor is opened, edited, and cancelled
- **THEN** the next drag still draws the ramp the tool had before the editor opened

#### Scenario: A preset replaces the edited ramp [gt_preset_replaces]

- **WHEN** a built-in preset is chosen from the sample's menu after an edited gradient was accepted
- **THEN** the next drag draws that preset and the edited ramp is forgotten
