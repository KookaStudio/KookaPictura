# ui/panel-refresh-lifecycle Specification

## Purpose

Defines when the shell re-binds docked panels to the active document surface and
when it refreshes their contents, so one edit updates every panel once without
rebuilding the dock.

## Requirements

### Requirement: Dock rebinding follows the active document surface

The shell SHALL bind docked panels to the active document view and canvas, and
SHALL rebind them only when the active view, the active canvas, or the document
generation changes. A surface change with no filter commit running on the new
view MUST rebind immediately; a running filter commit defers the bind until that
commit lands. An edit or preview on an unchanged surface MUST refresh panel
contents without re-binding the view. Closing a document that is neither the
active surface nor the surface the dock is bound to MUST NOT rebind.

#### Scenario: A document surface change rebinds the dock [panel_surface_change]

- **WHEN** the active document, its active window canvas, or its document
  generation changes and no filter commit is running on the new view
- **THEN** every docked panel is bound to the new surface and refreshed once
- **AND WHEN** a filter commit is running on the new view
- **THEN** the bind is deferred until that commit lands

#### Scenario: An edit or preview on the same surface refreshes in place [panel_edit_in_place]

- **WHEN** an edit commits or a region preview presents and the active view,
  active canvas, and document generation are unchanged
- **THEN** the docked panels refresh their contents without re-binding their view

#### Scenario: A committed edit does not rebind the dock [panel_no_rebind]

- **WHEN** a notifying edit commits on an unchanged document surface
- **THEN** no docked panel is re-bound to its view

#### Scenario: Closing an unbound inactive document does not rebind [panel_inactive_close]

- **WHEN** a document that is neither active nor bound to the dock is closed
- **THEN** no docked panel is re-bound and the dock rebind count is unchanged

#### Scenario: Closing the bound inactive document rebinds before freeing [panel_bound_close]

- **WHEN** a deferred bind leaves the dock bound to a document that is not
  active, and that document is closed
- **THEN** the dock is rebound before the bound view is freed

### Requirement: One refresh per committed edit

For a committed edit whose outcome notifies, the shell's debounced panel pass
SHALL refresh each docked panel at most once. A panel that already refreshes
itself from that edit's notification MUST NOT be refreshed again by the pass, so
the Layers panel rebuilds exactly once per edit.

#### Scenario: A committed edit refreshes Layers once [panel_layers_once]

- **WHEN** a notifying edit commits on an unchanged document surface
- **THEN** the Layers panel refreshes exactly once and the debounced pass does not
  rebuild it again

#### Scenario: A background change does not suppress the active panels [panel_background_change]

- **WHEN** a background document emits a change notification and then the active
  document commits a notifying edit
- **THEN** the active document's Layers panel still refreshes exactly once for
  that edit

### Requirement: A region-only stroke commit refreshes dependent panels

A committed paint stroke presents through the region path without a document
notification. After that commit, the debounced panel pass SHALL refresh the
panels whose contents depend on the document pixels or the history: Layers,
History, Histogram, Navigator, and Channels. Their contents MUST reflect the
committed pixels and the new history state.

#### Scenario: A stroke commit still refreshes the panels [panel_stroke_refresh]

- **WHEN** a paint stroke commits through the region path
- **THEN** after the debounce the Layers, History, Histogram, Navigator, and
  Channels panels reflect the new pixels and history state
