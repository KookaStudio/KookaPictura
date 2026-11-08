## ADDED Requirements

### Requirement: Default shortcuts follow the CS6 set

The command catalogue SHALL carry the CS6 default binding for every implemented
command that CS6 binds, and no two catalogue rows SHALL share a default chord.
The function-key panel toggles SHALL be Brush `F5`, Color `F6`, Layers `F7`,
Info `F8`, and Actions `F9`.

#### Scenario: Function keys toggle the panels [ds_function_key_panels]

- **WHEN** the window has focus and `F6`, `F7`, or `F8` is pressed
- **THEN** the Color, Layers, or Info panel's toggle command fires and its
  checked state flips

#### Scenario: Default chords are unique [ds_unique]

- **WHEN** the default shortcut of every catalogue row is collected
- **THEN** no chord appears on two rows

### Requirement: The CS6 command keeps a shared chord

A chord the CS6 contract assigns SHALL belong to the CS6 command. `Ctrl+Y`
SHALL be `View > Proof Colors` and `Ctrl+Alt+Shift+E` SHALL be
`Layer > Merge > Stamp Visible`. A command outside the CS6 set (`Redo`,
`Export As…`) SHALL carry no default binding for a chord CS6 assigns elsewhere,
so the menu and the shortcut editor never show an ambiguous chord.

#### Scenario: Proof Colors owns Ctrl+Y [ds_proof_colors]

- **WHEN** the catalogue is read
- **THEN** `view.proof.colors` is `Ctrl+Y` and `edit.redo` has no default

#### Scenario: Stamp Visible owns Ctrl+Alt+Shift+E [ds_stamp_visible]

- **WHEN** the catalogue is read
- **THEN** `layer.stamp.visible` is `Ctrl+Alt+Shift+E` and `file.exportAs` has
  no default
