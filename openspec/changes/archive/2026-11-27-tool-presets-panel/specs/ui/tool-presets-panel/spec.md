# ui/tool-presets-panel Specification

## Purpose

Give users a dock panel over the shared Tools preset list: capture a tool and
its options-bar settings, browse and apply presets for any tool, filter and
sort the list, and manage `.tpl` and native libraries.

## ADDED Requirements

### Requirement: Tool Presets panel surface

The application SHALL expose `Window > Panels > Tool Presets` as a checkable
command that shows and hides a Tool Presets panel. The panel SHALL list the
current Tools preset list in stored order with each row's name and owning tool,
and SHALL be hidden in the default Essentials layout. The Painting workspace
SHALL host the panel.

#### Scenario: The Window command toggles the panel [tpp_toggle]

- **WHEN** the user triggers `Window > Panels > Tool Presets`
- **THEN** the panel becomes visible and the command is checked, and triggering it again hides the panel

#### Scenario: The Painting workspace hosts the panel [tpp_painting_workspace]

- **WHEN** the user applies the Painting workspace on a fresh session
- **THEN** the Tool Presets panel is hosted in that workspace

### Requirement: Create and apply tool presets

The panel SHALL offer Create New Tool Preset, which SHALL capture the active
tool and its current options-bar settings, prompt for a name, and append the
preset to the shared Tools list. Selecting a preset SHALL switch to its owning
tool when different and apply the captured options, including type-tool
settings. Applying a preset SHALL NOT add a document history state.

#### Scenario: Create captures the active tool and options [tpp_create]

- **WHEN** the user configures a tool and creates a named tool preset
- **THEN** the shared Tools list contains a preset with that name, the owning tool, and the current option values, and the panel lists it

#### Scenario: Applying a preset switches tools [tpp_apply_switches]

- **WHEN** the user selects a preset whose owning tool differs from the active tool
- **THEN** the owning tool becomes active and the options bar shows the preset's captured values

#### Scenario: A type preset restores type settings [tpp_apply_type]

- **WHEN** the user applies a type-tool preset created with a font, size, justification, and antialias value
- **THEN** the Type options report every captured value

#### Scenario: Applying a preset records no history [tpp_apply_no_history]

- **WHEN** the user applies a tool preset with a document open
- **THEN** the document history count and dirty state are unchanged

### Requirement: Current Tool Only, Sort By Tool, and display modes

The panel SHALL provide a Current Tool Only checkbox and an equivalent menu
toggle, a Sort By Tool menu toggle, and Text Only, Small List, and Large List
display modes. Filtering, sorting, and display SHALL NOT change the stored
preset order.

#### Scenario: Current Tool Only filters the list [tpp_filter]

- **WHEN** Current Tool Only is on and the active tool has presets
- **THEN** the panel lists only presets owned by the active tool, and shows an empty list when that tool has none

#### Scenario: Sort By Tool groups the list [tpp_sort]

- **WHEN** Sort By Tool is on
- **THEN** the panel lists presets grouped by owning tool while preserving the stored order within each tool

#### Scenario: Display modes change the list presentation [tpp_display]

- **WHEN** the user chooses Text Only, Small List, or Large List
- **THEN** the panel shows the same presets with the corresponding text-only or icon presentation

### Requirement: Rename, delete, and reorder presets

The panel SHALL offer Rename Tool Preset and Delete Tool Preset for the current
row. Edits SHALL update the shared Tools list so the options-bar picker and the
Preset Manager reflect them. Drag reordering in the panel SHALL persist in the
current list and survive a restart.

#### Scenario: Rename and delete reach every consumer [tpp_rename_delete]

- **WHEN** the user renames and then deletes a tool preset
- **THEN** the panel, the options-bar picker, and the Preset Manager show the new name and then no longer list the deleted preset

#### Scenario: Reordering survives a restart [tpp_reorder_persist]

- **WHEN** the user reorders presets in the panel and the application restarts
- **THEN** the current Tools list is restored in the new order

### Requirement: Tool preset library commands

The panel menu SHALL offer Load Tool Presets (append), Replace Tool Presets,
Reset Tool Presets, Save Tool Presets, and a Preset Manager entry. Load and
Replace SHALL accept native `.kpset` and `.tpl` libraries; Save SHALL write the
native `.kpset` format. A supported library in the per-type discovery folder
SHALL be offered after restart. A missing or invalid library SHALL report an
error and SHALL NOT silently empty the current list.

#### Scenario: Load, Replace, Reset, and Save have distinct effects [tpp_load_replace_reset_save]

- **WHEN** the user saves the current Tools list, resets it, and loads the saved library
- **THEN** Save writes a native library, Reset restores the defaults, and Load appends the saved presets

#### Scenario: Discovered libraries appear in the panel menu [tpp_discovery]

- **WHEN** a supported Tools library is placed in the discovery folder and the application restarts
- **THEN** the panel menu offers that library for append or replace

#### Scenario: An invalid library preserves the current list [tpp_invalid]

- **WHEN** Load or Replace is given an invalid or truncated library
- **THEN** the panel reports the failure and the current Tools list is unchanged

### Requirement: The options-bar picker shares the Tools list

Every tool's options-bar Tool Preset picker SHALL list the current Tools presets
whose owning tool matches that tool and SHALL apply the supported payload.
Brush and Pencil SHALL keep their separate brush-preset entries, and an empty
match SHALL show a disabled `No Tool Presets` entry.

#### Scenario: Every tool's picker lists its presets [tpp_picker]

- **WHEN** a preset exists for a non-Brush tool and that tool is active
- **THEN** its options-bar picker lists the preset and applying it restores the captured options

### Requirement: Native presets preserve the full options snapshot

A native Tools preset SHALL retain the complete captured tool-options object
alongside the owning tool identifier and the mapped `.tpl` values. Unknown
option fields SHALL survive a save and re-save.

#### Scenario: A native preset round-trips every option [tpp_native_round_trip]

- **WHEN** a preset created for a tool with several option groups is saved and read back
- **THEN** applying it restores every captured value

#### Scenario: Unknown option fields survive a re-save [tpp_unknown_options]

- **WHEN** a native Tools preset contains an option field the current build does not know
- **THEN** saving the list again preserves that field
