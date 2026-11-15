# ui/main-window-collaborators Specification

## Purpose
The application shell's main window composes owned collaborators that each hold
one shell concern — documents and their windows, panels and workspaces, file
open/save/import, and command registration — so panel and menu changes no longer
need the main-window header and each collaborator is testable on its own.

## Requirements

### Requirement: The main window delegates owned state to collaborators

The main window SHALL NOT hold the primary state for open documents and document
windows, panels/panel groups/panel columns/workspaces, file open/save/import and
recovery bookkeeping, or the command-handler table. Those SHALL be owned by
`DocumentRegistry`, `PanelHost`, `ImportSaveCoordinator`, and `MenuRegistrar`
respectively. The main window SHALL retain only its `QMainWindow` overrides,
chrome and settings state, the remaining tool-shell state (`ToolController`,
toolbox, options bar), the composition root and signal wiring, the close entry
point that forwards to `ImportSaveCoordinator`, and test hooks that forward to
the owning collaborator. A collaborator SHALL NOT
implement its behavior by calling back into the main window; a forwarder that
only re-exposes the collaborator's methods SHALL NOT count as the collaborator
owning the concern.

#### Scenario: Frame hooks forward to the owning collaborator [mwc_forwarders_delegate]

- **WHEN** a document is created through the composition root and the frame's document hooks are read
- **THEN** the values come from `DocumentRegistry`, and a change made directly through the registry is visible through the same hooks

#### Scenario: Removing a document notifies its owners [mwc_removal_notifies]

- **WHEN** a smart-object editor tab or a recovered document is closed
- **THEN** recovery, open-token, edit-session, dialog, and tool cleanup runs before the document is destroyed

#### Scenario: No collaborator is a frame facade [mwc_no_facade]

- **WHEN** a production translation unit of `DocumentRegistry`, `PanelHost`, `ImportSaveCoordinator`, or `MenuRegistrar` is inspected
- **THEN** it contains no `PicturaMainWindow` reference and no `frame.h` include

### Requirement: Each collaborator is constructible without the main window

Each collaborator SHALL be constructible from its declared dependencies — the
widgets, session values, and other collaborators it needs — without a
`PicturaMainWindow` instance. A Qt Test SHALL exercise the collaborator's core
behavior in that configuration, so the collaborator is reachable without the
whole shell.

#### Scenario: DocumentRegistry operates alone [mwc_registry_standalone]

- **WHEN** a `DocumentRegistry` is constructed with a parent widget and no main window
- **THEN** adding, activating, floating, docking, tiling, reordering, and removing documents through it succeeds and reports the document/active-window changes

#### Scenario: PanelHost operates alone [mwc_panel_host_standalone]

- **WHEN** a `PanelHost` is constructed with a stub document area and no main window
- **THEN** it builds the panel set, creates and moves panel columns, toggles panels, and applies a workspace layout

#### Scenario: ImportSaveCoordinator operates alone [mwc_files_standalone]

- **WHEN** an `ImportSaveCoordinator` is constructed with a `DocumentRegistry` and no main window
- **THEN** it saves a document to a temporary PSD/PSB, opens it again, and records the path in the recent list

#### Scenario: MenuRegistrar operates alone [mwc_menu_standalone]

- **WHEN** a `MenuRegistrar` is constructed with the collaborators, a stub chrome registrar, and a stub `MenuRuntime`, and no main window
- **THEN** it builds the menu bar and reports the enabled/checked state of a document-dependent command

### Requirement: Panel and menu code does not depend on the frame header

No production translation unit under `crates/pictura-app/cpp/panels/` and no
command/menu translation unit that belongs to `MenuRegistrar` SHALL include
`frame.h` or name `PicturaMainWindow`. A collaborator header SHALL NOT include
`frame.h`. Frame-owned chrome commands MAY be registered by the composition
root, but that registration SHALL be passed to `MenuRegistrar` as a callback so
the registrar's own translation units stay free of the frame.

#### Scenario: The panel tree is frame-free [mwc_panels_no_frame]

- **WHEN** the production files under `crates/pictura-app/cpp/panels/` are searched
- **THEN** none includes `frame.h` and none names `PicturaMainWindow`

#### Scenario: The menu registrar is frame-free [mwc_menus_no_frame]

- **WHEN** the `MenuRegistrar` translation units are searched
- **THEN** none includes `frame.h` and none names `PicturaMainWindow`

#### Scenario: Collaborator headers stay frame-free [mwc_collab_headers_clean]

- **WHEN** the collaborator headers are searched
- **THEN** none includes `frame.h`

### Requirement: The shell seam ratchet uses fixed ceilings

A repository check SHALL fail when a checked panel or menu translation unit
includes `frame.h` or names `PicturaMainWindow`, when any `frame_menus*.cpp`
remains, when a collaborator translation unit names `PicturaMainWindow` or
includes `frame.h`, when a collaborator header includes `frame.h`, when `frame.h`
exceeds 760 lines, or when `frame_includes.h` exceeds 60 include lines. The
ceilings SHALL NOT increase, there is no allowlist, and the check SHALL run from
`scripts/verify-fast.sh`.

#### Scenario: A panel frame dependency fails the gate [mwc_guard_panels]

- **WHEN** a checked panel translation unit gains a `frame.h` include or a `PicturaMainWindow` reference
- **THEN** `scripts/verify-fast.sh` fails and names the file

#### Scenario: A menu frame dependency fails the gate [mwc_guard_menus]

- **WHEN** a checked menu translation unit gains a `frame.h` include or a `PicturaMainWindow` reference, or a `frame_menus*.cpp` remains
- **THEN** `scripts/verify-fast.sh` fails and names the file

#### Scenario: Header growth fails the gate [mwc_guard_budgets]

- **WHEN** `frame.h` grows past 760 lines or `frame_includes.h` grows past 60 include lines
- **THEN** `scripts/verify-fast.sh` fails

### Requirement: Existing shell behavior is unchanged

The change SHALL NOT alter user-visible shell behavior or a persisted format:
document open/save/close, window hosts, panel/workspace layout, menus and
commands, recovery, and the session store SHALL behave as before. Every
registered Qt suite SHALL pass without edits to anything under
`crates/pictura-app/cpp/tests/`; the panel test-support TU
`panels/panel_column_test.cpp` may change. Both headless self-test invocations
SHALL emit the same tokens and exit codes, and the session schema and values
SHALL round-trip unchanged.

#### Scenario: Existing suites pass unchanged [mwc_existing_suites]

- **WHEN** the full test report runs after the refactor
- **THEN** every registered Qt suite passes and no file under
  `crates/pictura-app/cpp/tests/` was edited

#### Scenario: Self-test output is unchanged [mwc_selftest_baseline]

- **WHEN** both headless self-test invocations run after the refactor
- **THEN** their pass/fail/skip tokens and exit codes match the pre-change baseline

#### Scenario: Session state round-trips [mwc_session_roundtrip]

- **WHEN** the shell saves and restores session state after the refactor
- **THEN** the schema version, keys, and values match the pre-change store for the same settings
