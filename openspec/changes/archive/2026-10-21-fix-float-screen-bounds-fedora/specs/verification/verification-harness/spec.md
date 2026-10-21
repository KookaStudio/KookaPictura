# Spec Delta: fix-float-screen-bounds-fedora

## MODIFIED Requirements

### Requirement: Placement-independent screen-relative self-tests

A headless self-test that measures a floating overlay against the screen SHALL
NOT depend on the main window's size or position, and SHALL NOT require a band
between the window and the screen edge, because the window's minimum size
exceeds the offscreen screen on every platform and such a band does not exist.
The check SHALL place the main window at a known position, move the overlay
past the screen's bottom-right corner, and pass only when the overlay comes
back clamped inside the screen's available geometry and resting on the
screen's bottom-right edge, which is what tells a screen-relative clamp apart
from a window-relative one.

#### Scenario: The float screen-bounds check ignores default window placement [vh_float_screen_bounds]

- **WHEN** the `float_screen_bounds` self-test check runs under the offscreen QPA
  from any initial frame position and with any frame size
- **THEN** it places the frame at the screen origin, moves the float past the
  screen's bottom-right corner, and passes only when the float is clamped
  inside the screen's available geometry and rests on the screen's bottom-right
  edge
