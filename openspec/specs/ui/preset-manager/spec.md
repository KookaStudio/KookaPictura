# ui/preset-manager Specification

## Purpose

Give users one place to manage the current application-wide preset sets and use the same items in their owning pickers, dialogs and panels.

## Requirements

### Requirement: Eight managed preset types

`Edit > Presets > Preset Manager` SHALL open without an active document. Its type selector SHALL list Brushes, Swatches, Gradients, Styles, Patterns, Contours, Custom Shapes, and Tools. The Manager SHALL offer Text Only, Small Thumbnail, Large Thumbnail, Small List and Large List views, plus Stroke Thumbnail for Brushes. It SHALL support selection, multi-selection, Unicode rename, removal, and drag reorder. Each row SHALL have a stable identity independent of the name or current position.

#### Scenario: The Manager opens with the documented pages and views [preset_ui_pages]

- **WHEN** the Manager is opened without a document and the user switches between Brushes and another page
- **THEN** the selector lists the eight types and Stroke Thumbnail is offered only for Brushes

#### Scenario: List edits target the selected identities [preset_ui_edit]

- **WHEN** the user reorders, renames, and deletes items from a list containing duplicate names
- **THEN** the intended items are changed, the remaining order is correct, and any connected picker presents the same list

### Requirement: Library commands commit complete changes

Load SHALL append a library in file order. Replace SHALL substitute the current list. Reset SHALL offer replacing the list with defaults or appending defaults. Discovered libraries SHALL offer append or replace. A partial result SHALL present the diagnostic and require confirmation before applying the usable subset. A valid explicitly empty native set SHALL clear the current list on Replace and leave it unchanged on Load. Cancellation, failure, or a partial import with no usable entries SHALL preserve the current set. Load and save SHALL keep the Manager responsive.

#### Scenario: Load, Replace and Reset have distinct effects [preset_ui_library_commands]

- **WHEN** the user loads and replaces with a valid library, then resets in each of the offered modes
- **THEN** the current list is appended to, substituted, or reset in the requested way, preserving the respective source order

#### Scenario: Partial import and failure leave a predictable list [preset_ui_import_errors]

- **WHEN** the user rejects or accepts a partial library, cancels an import, or uses an invalid file with Replace
- **THEN** only accepting the partial library changes the list, and the result contains the reported usable entries in order

#### Scenario: A late import cannot overwrite a newer choice [preset_ui_async]

- **WHEN** a pending import is cancelled or superseded while the user continues to interact with the Manager
- **THEN** the Manager accepts the interaction before the old work completes and the old result cannot update the list

#### Scenario: An empty native set can replace a populated list [preset_ui_empty_set]

- **WHEN** an empty native set is saved and then loaded or used to replace a populated list
- **THEN** Load leaves the list unchanged and Replace clears it, while a failed or partial zero-usable import preserves it

### Requirement: Save Set and preset persistence

Save Set SHALL save the selected entries in library order, or the whole set when no items are selected. It SHALL offer native export and the supported Adobe writers for the current type. File writes SHALL be atomic. Current list contents, including an explicitly empty list and its order, SHALL survive restart. Existing swatch settings SHALL migrate on first read without losing custom names, colors, order, or intentional emptiness. A supported library in a per-type discovery directory SHALL be offered after startup. Preference reset SHALL warn that edited current lists will be removed, publish a complete candidate generation of default current lists by atomically switching one active manifest before reporting success, and SHALL keep exported libraries. A failed reset write SHALL report failure and preserve the previous current lists and legacy swatch data, so reset cannot resurrect legacy swatches on a later launch.

#### Scenario: Save Set reloads the selected records [preset_ui_save]

- **WHEN** the user saves a non-contiguous selection or a page with no selection, and a subsequent export fails before commit
- **THEN** reloading each successful file yields the selected records in list order or the entire current set as appropriate, and the failed export preserves the prior file

#### Scenario: Current lists migrate and survive a restart [preset_ui_persist]

- **WHEN** old swatches are restored, a list is renamed and reordered, and another is emptied before restarting
- **THEN** the migrated swatches and subsequent list changes survive with the exact names and order, and the empty page stays empty

#### Scenario: Discovery and preference reset honor the library location [preset_ui_discovery_reset]

- **WHEN** a type directory contains a valid library and the user restarts then confirms a preference reset
- **THEN** the library is offered on its matching page, the warning precedes removal of the current-list edits, and the external library is still available

#### Scenario: Reset after migration stays reset [preset_ui_migration_reset]

- **WHEN** legacy custom swatches are migrated, preferences are reset, and the application restarts
- **THEN** the default swatches remain current rather than the legacy custom swatches; a failed reset write preserves the previous list and legacy data

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
