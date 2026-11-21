# UI Localization Spec Delta

## Purpose

Let users choose the language of the application interface and install their
own translations independently of the system's numeric locale.

## ADDED Requirements

### Requirement: Bundled interface catalogs

Release and development builds SHALL include a separate catalog for German and
Finnish. Each catalog SHALL cover app-owned presentation text in the Qt shell,
including menus, toolbox and flyouts, the Options bar for every tool, dockable
panel tabs and contents, dialogs, and status and error messages. Platform-owned
controls, document content, user names, and external names follow their existing
sources. Labels SHALL remain independent of command IDs, panel IDs, control
values, and persisted formats. English source text SHALL be used for every
message absent from the active catalog.

#### Scenario: Both bundled languages translate the agreed surfaces [loc_bundled]

- **WHEN** the app starts with System selected and the system requests German
  or Finnish, including a regional variant such as `de-AT` or `fi-FI`
- **THEN** the corresponding bundled catalog translates menu labels and
  commands, tool names and tooltips, and the agreed common dialogs

#### Scenario: Panel tabs and their contents use the selected language [loc_panel_contents]

- **WHEN** the shell opens under German or Finnish and shows a dockable panel
- **THEN** its tab and contents use the active language, and panel identity in
  session layout and Window-menu dispatch stays stable

#### Scenario: Tool options use the selected language [loc_tool_options]

- **WHEN** the Options bar displays a tool under German or Finnish
- **THEN** labels, choices, and tooltips use the active language while the
  controls continue to submit the same tool settings

#### Scenario: Additional dialogs use the selected language [loc_additional_dialogs]

- **WHEN** a standalone dialog for an editing command opens under German or Finnish
- **THEN** its app-owned controls use the active language while choices retain
  their original semantic values

#### Scenario: System uses English when preferred or when no catalog matches [loc_unsupported]

- **WHEN** System is selected and either the OS prefers English to other
  supported languages or it supplies no matching language
- **THEN** the app displays English source text

#### Scenario: Missing translations and command identities [loc_source_fallback]

- **WHEN** the active catalog has no translation for a displayed action and
  another action has a translation
- **THEN** the first action displays its English label and the translated
  action still dispatches through its original command identity and shortcut

#### Scenario: Dynamically populated menus retain their identity [loc_dynamic_menus]

- **WHEN** the interface is German or Finnish and recent files and workspaces
  are available
- **THEN** Open Recent and Workspace populate their entries and dispatch the
  chosen entry using the original menu and command identities

### Requirement: Language selection applies at startup

Preferences → Interface → Language SHALL offer System, English, Deutsch,
Suomi, and successfully imported translations. System SHALL select the first
supported language in the OS's ordered UI language preferences, counting
source English as supported. An explicit selection SHALL override the system
UI language. The app SHALL persist the selection,
tell the user that a change takes effect on the next launch, and establish the
selected language before creating the interface. Existing preferences without a
language selection and unavailable explicit selections SHALL use System.

A changed selection SHALL become the pending choice only after a confirmed
preferences save. The write SHALL preserve unrelated preferences. If saving
fails, the selector SHALL restore the last saved choice, show an error, and
omit the restart success notice. The active language SHALL remain unchanged.

The Language row SHALL show a restart button while a saved choice differs from
the choice applied at startup. The restart SHALL follow the existing document
save and discard workflow. A cancelled restart or failed launch SHALL leave
the running app available.

The UI language SHALL be independent of the system numeric locale.

#### Scenario: An explicit choice applies on the next launch [loc_explicit]

- **WHEN** the system UI language is German and the user selects Suomi in
  Preferences
- **THEN** the running interface remains German and displays restart guidance,
  and the next launch uses Finnish

#### Scenario: Explicit English and switching back to System [loc_restore_system]

- **WHEN** the system UI language is Finnish, the user selects English and
  relaunches, and later selects System and relaunches again
- **THEN** the first relaunch uses source English and the second uses Finnish

#### Scenario: Restart applies a pending language [loc_restart_button]

- **WHEN** the user saves a different language and activates the restart button
- **THEN** the app runs the existing document save checks and launches a new
  instance with the saved choice, or stays open if the restart is cancelled
  or the new instance cannot launch
- **AND** the button is hidden on the new launch and when the user saves the
  startup choice again

#### Scenario: Numeric locale is independent of UI language [loc_independent]

- **WHEN** the app starts with English selected and a German or Finnish system
  numeric locale
- **THEN** the interface text is English and in-scope numeric fields and
  readouts use the locale's comma decimal separator

#### Scenario: A failed language save preserves the last choice [loc_save_failure]

- **WHEN** the user changes the language and the preferences store cannot
  save the change
- **THEN** the selector returns to the last saved choice, an error is shown, no
  restart success notice appears, and the active language remains unchanged
- **AND** a later save of an unrelated preference retains the last saved language

### Requirement: Deterministic existing self-test

The existing self-test SHALL run with English UI and C numeric locale regardless
of the launch environment, preserving its existing checks and tokens. Localized
startup SHALL be covered by Qt Test without adding self-test tokens.

#### Scenario: Self-test under a non-English environment [loc_selftest_locale]

- **WHEN** the existing self-test starts under non-English UI and comma-decimal
  numeric locale environment settings
- **THEN** its existing checks use English UI and C numeric formatting and pass

### Requirement: Import a local community catalog

Preferences → Interface → Language SHALL provide **Import Translation** for a
local Qt Linguist source file (`.ts`). The supported entries SHALL be finished,
context-based, singular messages identified by their English source text and
optional disambiguation. Unfinished and unsupported entries SHALL remain eligible
for English source fallback. The app SHALL list a successfully imported catalog
as a separate language choice and retain it in the user's app data independently
of the source file's location. Import alone SHALL leave the selected language
unchanged.

The app SHALL validate the file before publishing it. A malformed, incompatible,
unreadable, or oversized file SHALL show an import error and preserve existing
catalogs and the selected language. If the chosen import cannot be used at
startup, the app SHALL continue with the System choice. The import flow SHALL
accept data from the selected file without resolving references to other files
or network resources.

Translated text SHALL render as plain text at plain-text display sites. At sites
with intentional app-owned markup, the application SHALL control the markup
and escape translated text before inserting it.

#### Scenario: Imported markup renders as text [loc_import_plain_text]

- **WHEN** a valid imported catalog supplies rich-text-looking content for a
  plain Preferences label
- **THEN** the label displays the literal content without interpreting markup

#### Scenario: Import and select a community translation [loc_import]

- **WHEN** a valid partial community catalog is imported, the original file is
  removed, and the catalog is selected in Preferences
- **THEN** the current interface stays in its current language, the next launch
  uses the installed catalog even when its language differs from the system
  language, and missing entries display English source text

#### Scenario: A failed import preserves the current choice [loc_import_invalid]

- **WHEN** the file picker returns a malformed, incompatible, unreadable,
  oversized, or externally referencing catalog
- **THEN** an import error is shown, no new catalog is selectable, and the
  previous selection and installed catalogs remain intact

#### Scenario: An unavailable import falls back to System [loc_import_missing]

- **WHEN** the selected imported catalog is missing or unusable at startup
- **THEN** the app opens with the bundled catalog selected by the system UI
  language, or English if there is no match
