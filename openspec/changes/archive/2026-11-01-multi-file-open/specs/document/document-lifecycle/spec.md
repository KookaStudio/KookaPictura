## MODIFIED Requirements

### Requirement: Open documents

The system SHALL open one or more files chosen from a file dialog into new
documents, and SHALL offer the most recently opened files through an Open Recent
list. The `File > Open` dialog SHALL accept a multi-file selection, including a
Shift-click range and Ctrl/Cmd-click individual files, and SHALL open one
document per selected file. A file that fails to open SHALL NOT prevent the
remaining selected files from opening. Each document SHALL follow the
document-window host rules, including the `Open Documents As Tabs` preference. A
successful open SHALL leave the document unmodified with its path recorded.

#### Scenario: Open a file adds a document

- **WHEN** the user opens a valid PSD
- **THEN** a new tab shows the composited document and the command line argument path is recorded

#### Scenario: Open failure does not add a document

- **WHEN** the chosen file cannot be read
- **THEN** no tab is added and the application remains usable

#### Scenario: Recent files

- **WHEN** a document is opened successfully
- **THEN** it appears at the front of the bounded Open Recent list

#### Scenario: Multi-select opens one document per file [dlg_open_multiple]

- **WHEN** the user selects several files in `File > Open` and confirms
- **THEN** one document opens for each selected file, each records its own path,
  and every document follows the tabs preference

#### Scenario: A failed file does not stop the rest [dlg_open_multiple_partial]

- **WHEN** the selection includes a file that cannot be read and other readable
  files
- **THEN** the readable files open and the failed file adds no document

#### Scenario: A single selection still opens one document [dlg_open_single]

- **WHEN** the user selects one file in `File > Open` and confirms
- **THEN** one document opens for it
