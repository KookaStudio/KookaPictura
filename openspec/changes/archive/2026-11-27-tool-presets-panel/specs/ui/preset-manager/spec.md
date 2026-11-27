# ui/preset-manager Specification

## MODIFIED Requirements

### Requirement: Application-wide lists feed preset consumers

The Manager and each supported consumer SHALL use the same current list. The
Swatches panel SHALL use that list for selecting a foreground or background
color and for its library menu commands. The brush picker, gradient controls,
Layer Style and contour controls, pattern consumers, Custom Shape picker, and
tool-preset picker SHALL apply the supported payload of the chosen entry. The
tool-preset picker SHALL list the current Tools presets for every tool, not only
Brush and Pencil, and the Manager's Tools page SHALL run Load, Replace, Reset,
and Save for that same list. The current lists SHALL be available across
windows. Managing a list SHALL create no document History entry. Applying an
entry SHALL follow the owning tool or dialog's normal commit and cancel
behavior. A gesture already in progress SHALL retain its captured preset values.
Document content created with a preset SHALL retain its appearance after the
library changes and the document reopens.

#### Scenario: Imported presets produce changes in their owning consumers [preset_ui_apply]

- **WHEN** one usable external entry of each managed type is selected in its owning control and applied through the normal workflow
- **THEN** each color, stroke, gradient, effect, pattern, contour, shape or tool option reflects the imported payload

#### Scenario: A library edit does not rewrite an in-progress stroke or applied content [preset_ui_snapshot]

- **WHEN** the selected entry is edited or removed while a brush stroke holds its captured settings or after a document embeds an imported asset
- **THEN** the captured stroke and reopened document retain their previous appearance and the library edit itself adds no History state

#### Scenario: Every tool consumes the Tools list [preset_ui_tools_picker]

- **WHEN** a Tools preset exists for a tool other than Brush or Pencil and that tool's options-bar picker is opened
- **THEN** the preset is listed and applying it changes that tool's options

#### Scenario: The Tools page runs the library commands [preset_ui_tools_library]

- **WHEN** the Preset Manager is opened on the Tools page and Load, Replace, Reset, or Save is invoked
- **THEN** the Tools current list is appended to, replaced, reset, or written as a native library through the same commands the panel uses
