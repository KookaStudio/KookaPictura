# Design: show-transform-controls

## Context

The Move tool (`tool_move.cpp`) owns translation with a cached base+layer
preview. The Free Transform session already provides the bounding box, handle
hit-testing, and scale/rotate/move gestures, but it is modal: the options bar
switches to the transform page and Enter/Escape commit or cancel. `Show
Transform Controls` needs the box without the modal command.

## Key decisions

### An auto-managed controls session

`ToolController` gains a `transformControls_` flag and `syncTransformControls()`,
called when the tool changes or the canvas rebinds (`applyToolPolicy`,
`bindCanvas`), when the checkbox toggles, when the Layers panel selection
changes, and on every document refresh. The sync:

- does nothing while a user (Ctrl+T) session owns the overlay;
- hides the box when the tool is not Move, the checkbox is off, or a selection
  exists;
- (re)begins a Free Transform session on the active layer when the box is
  missing or stale — stale meaning the session's target or its content rect no
  longer matches the layer (a content edit, an undo, a layer change). The box
  is rebuilt rather than left pointing at old pixels;
- never begins on an untransformable target (`layer_can_free_transform`), so a
  Background, group, adjustment, or locked layer shows no box;
- hides the reference point (`transform_set_show_pivot(*v, false)`), matching
  CS6.

A new bridge readout `transform_session_rect` (the session's content rect)
drives the staleness check.

### Gestures: handles commit per drag, inside presses are Move drags

`handlePressed` in a controls session:

- a handle (0..=7) or rotate-band (8) hit starts the session gesture as usual;
- a move hit (9) or no hit cancels the controls session and falls through to
  the Move handler, so translation keeps its cached preview, integer delta,
  `"Move Layer"` history, Alt-duplicate, and cross-document drag. The box
  returns on release (the release path re-syncs).

`handleReleased` in a controls session applies the gesture immediately: the
session commits (one `"Free Transform"` state when anything changed; an
identity drag records nothing), the overlay clears, and the sync rebuilds the
box on the resulting layer.

`Ctrl+T` cancels a controls session before beginning the user session, so the
same-path early-return in `begin_transform_session` cannot swallow it, and the
options bar switches to the transform page.

### The options bar

The `transformChanged` handler keeps the Move page while
`transformControlsSession()` is true; otherwise it switches to the transform
page as before. The checkbox keeps its existing `moveOptionsChanged` wiring and
now also calls the sync.

## Risks

- The sync runs on every `bindCanvas` (every refresh). It is cheap while the
  checkbox is off and only rebuilds the box when the target or rect changed.
- Beginning a controls session trims the layer to its content (the existing
  Free Transform begin), a silent rect change with no history state. Accepted
  as the existing session behavior.
