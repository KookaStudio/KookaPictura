# Spec Delta

## Purpose

`File > Browse…` gives Linux users the desktop-browsing entry that CS6 provided
through Adobe Bridge, without making Bridge a dependency.

## ADDED Requirements

### Requirement: Browse command and menu placement

`File > Browse…` (`file.browse`) SHALL appear in the File menu immediately after
`Open` and before `Open As`, SHALL use `Ctrl+Alt+O`, SHALL be enabled with or
without an open document, and SHALL NOT require Adobe Bridge.

#### Scenario: The Browse row sits in the CS6 Bridge position [fb_menu]
- **WHEN** the File menu is opened with no document
- **THEN** `Browse…` appears between `Open` and `Open As`, carries `Ctrl+Alt+O`,
  and is enabled

#### Scenario: Browsing needs no Bridge [fb_no_bridge]
- **WHEN** `file.browse` is dispatched on a system without Adobe Bridge installed
- **THEN** the command performs its folder hand-off without launching or
  requiring Bridge

### Requirement: Browse target and side effects

Dispatching `File > Browse…` SHALL open the active document's containing folder
in the desktop file manager. With no document, or with an active document that
has never been saved, it SHALL open the home folder. The command SHALL NOT open,
close, create, or modify a document.

#### Scenario: A saved document's folder is opened [fb_document_folder]
- **WHEN** the active document was opened from a file and `file.browse` is
  dispatched
- **THEN** the desktop opener receives that file's containing folder

#### Scenario: No saved document opens home [fb_home]
- **WHEN** no document is open, or the active document is untitled, and
  `file.browse` is dispatched
- **THEN** the desktop opener receives the home folder

#### Scenario: Browsing leaves documents untouched [fb_unchanged]
- **WHEN** `file.browse` is dispatched while documents are open
- **THEN** every document's pixels, path, modified state, active view, and
  history are unchanged
