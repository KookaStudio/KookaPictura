# tools/hand-navigation-gestures Specification

## Purpose
Hand navigation can temporarily show the full document to choose a new location
at the current zoom, and can continue a fast Hand drag with bounded momentum.

## Requirements

### Requirement: Bird's Eye View relocates the current canvas

Holding an accepted plain H Hand tool shortcut and pressing the primary mouse button inside a canvas with a zoomed-in document SHALL display a fit overview of the entire document with a widget-space destination outline representing the previous viewport. Pointer movement SHALL move the outline and releasing the button SHALL return to the previous zoom and rotation centred on the chosen document location subject to the existing pan bounds. The gesture SHALL operate on the initiating canvas and SHALL NOT edit content or create a history entry.

#### Scenario: Relocate at the prior zoom [hand_bird_relocate]

- **WHEN** the user holds H, presses and drags over a zoomed-in canvas, and releases the button
- **THEN** the entire document appears in the fit overview, the visible outline follows the pointer and the view returns at the prior zoom and rotation with a new location and no tool event or history edit

#### Scenario: Cancel the overview [hand_bird_cancel]

- **WHEN** the user presses Escape, releases H early, loses canvas focus or has input interrupted during the overview
- **THEN** the overview disappears and the original viewport remains, including after the matching mouse release, which SHALL NOT reach the tool

#### Scenario: Plain pan remains available [hand_plain_pan]

- **WHEN** the user drags with the Hand tool selected after releasing H, or uses Space or the middle mouse button
- **THEN** the ordinary pan path is used without opening the overview

#### Scenario: H respects shortcut ownership [hand_bird_shortcuts]

- **WHEN** H is consumed by an active tool, text input or command, Hand is remapped away from H, or a held H produces autorepeat events
- **THEN** only an accepted initial plain H Hand shortcut arms the overview; repeats do not rearm or cancel it, and a remapped Hand shortcut selects Hand without arming this H-specific gesture

### Requirement: Flick Panning follows Hand motion

The General preference Flick Panning SHALL persist as Automatic, On or Off, defaulting to Automatic. Where GPU navigation is available, Automatic SHALL enable flick unless reduced motion is requested, On SHALL explicitly override reduced motion, and Off SHALL disable flick. When enabled, a fast primary-button Hand or temporary Space-Hand drag SHALL continue briefly after release. The motion SHALL decay, respect pan limits, and stop on new navigation input or view loss. A paused release SHALL stop immediately. When GPU navigation is unavailable, Hand panning SHALL end on release and the preference control SHALL be unavailable without changing its stored value. The feature SHALL NOT change document history.

#### Scenario: A fast drag continues and stops [hand_flick_motion]

- **WHEN** an eligible Hand drag releases at speed
- **THEN** the view moves farther in the drag direction, respects bounds, and interrupting input stops the motion without a history edit

#### Scenario: Ineligible drags have no momentum [hand_flick_gates]

- **WHEN** a Hand drag releases after a pause, with the option Off, with GPU navigation unavailable, or in Automatic while reduced motion is requested
- **THEN** no motion follows release

#### Scenario: Explicit reduced-motion override [hand_flick_override]

- **WHEN** reduced motion is requested and Flick Panning is explicitly set to On on a GPU-capable view
- **THEN** an eligible fast Hand drag continues after release

#### Scenario: The preference survives reopening settings [hand_flick_preference]

- **WHEN** each Flick Panning choice is saved and Preferences are reopened
- **THEN** the stored value is shown and CPU-only navigation presents the control as unavailable without changing that value

#### Scenario: Flick scrolls visible peer windows [hand_flick_peers]

- **WHEN** an eligible Hand drag releases at speed with Scroll All Windows enabled or Shift held
- **THEN** each inertial tick sends the initiating view's actually applied displacement to visible peers, which each respect their own bounds; without either condition the peers do not move
