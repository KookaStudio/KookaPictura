## ADDED Requirements

### Requirement: Initial document view settles at the real viewport

When a document or image is attached to the canvas, the view SHALL compute its
initial fit against the viewport the canvas has once it is laid out and shown:
zoom 100 % with the document centred when it fits the viewport, and the highest
zoom that fits (95 % of the viewport's limiting axis) with the document centred
when it does not. The view MUST NOT keep a fit computed against a pre-layout
viewport. An explicit user view change made before the settle ends SHALL be
kept by later resizes.

#### Scenario: A small new document is centred at 100 %

- **WHEN** a document smaller than the canvas viewport is created before the
  window is shown
- **THEN** once the window is shown and laid out, the canvas zoom is 100 % and
  the document centre maps to the viewport centre

#### Scenario: A large new document is fitted and centred

- **WHEN** a document larger than the canvas viewport is created before the
  window is shown
- **THEN** once the window is shown and laid out, the canvas zoom is the fit
  zoom for the real viewport size and the document centre maps to the viewport
  centre

#### Scenario: An explicit zoom survives later resizes

- **WHEN** the user sets a zoom after the initial settle and the window is
  then resized
- **THEN** the zoom is kept and only the offset is clamped into range
