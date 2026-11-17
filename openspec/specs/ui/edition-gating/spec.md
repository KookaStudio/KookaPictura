# ui/edition-gating Specification

## Purpose
Select the Photoshop CS6 edition in a single build and gate the surfaces that
differ between Standard and Extended.

## Requirements

### Requirement: Edition selection

The application SHALL read `PICTURA_EDITION`, accept `standard` and `extended`
case-insensitively, and default to Extended when the variable is unset or
unrecognized. The core SHALL expose the choice as `pictura_core::Edition`.

#### Scenario: Standard is selected
- **WHEN** `PICTURA_EDITION=standard`
- **THEN** the application runs as `Edition::Standard`

#### Scenario: Extended is the default
- **WHEN** `PICTURA_EDITION` is unset or holds an unknown value
- **THEN** the application runs as `Edition::Extended`

### Requirement: Standard hides Extended-only tools

In Standard the toolbox SHALL omit the Count tool, SHALL not select it through
the `I` group or `setActiveTool`, and SHALL keep the other tools in its group
available. In Extended the Count tool SHALL remain available.

#### Scenario: Count is absent in Standard
- **WHEN** the application runs as Standard and the `I` group is inspected
- **THEN** the Count tool is absent from the flyout and cannot become active

#### Scenario: Count is present in Extended
- **WHEN** the application runs as Extended and the `I` group is inspected
- **THEN** the Count tool is present and selectable

### Requirement: Standard hides Extended-only commands

In Standard the command registry SHALL omit the 3D commands, the Measurement
Log panel command, the Count overlay command, and the 12 stack-mode commands
under `layer.smart.objects.stack.mode.*`. It SHALL keep the Timeline command,
which ships in both CS6 editions. In Extended those commands SHALL remain
registered.

#### Scenario: Extended-only commands are absent in Standard
- **WHEN** the application runs as Standard
- **THEN** the 3D, Measurement Log and Count overlay commands have no action

#### Scenario: Timeline stays in Standard
- **WHEN** the application runs as Standard
- **THEN** the Timeline command is still registered

#### Scenario: Extended keeps the commands
- **WHEN** the application runs as Extended
- **THEN** the 3D, Measurement Log and Count overlay commands are registered

#### Scenario: Stack-mode commands are absent in Standard [edg_stack_absent]
- **WHEN** the application runs as Standard and then as Extended
- **THEN** the 12 `layer.smart.objects.stack.mode.*` commands have no action in Standard and are registered in Extended
