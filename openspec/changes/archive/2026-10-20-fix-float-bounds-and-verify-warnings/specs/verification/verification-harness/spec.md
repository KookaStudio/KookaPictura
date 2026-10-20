# Spec Delta

## ADDED Requirements

### Requirement: Placement-independent screen-relative self-tests

The headless self-test SHALL place the main window at a known position before a
check that requires a band between the window and the screen edge, so the
check's result does not depend on the platform's default window placement. The
check SHALL keep asserting that a floating overlay moved past the screen edge is
clamped inside the screen's available geometry and may sit outside the main
window's rect.

#### Scenario: The float screen-bounds check ignores default window placement [vh_float_screen_bounds]

- **WHEN** the `float_screen_bounds` self-test check runs under the offscreen QPA
  from any initial frame position
- **THEN** it places the frame at a known position where the available band
  exists, and it passes only when the float is clamped inside the screen and sits
  outside the frame

### Requirement: Warning-free verification output

The local fast gate, the local full gate, and a clean CMake build SHALL emit no
warnings caused by project code or configuration. A warning caused by a
deliberate private dependency, generated bindings, or third-party headers SHALL
be suppressed narrowly at the boundary with a documented reason rather than by
filtering the verification log. Warnings emitted only by third-party packaging
tools outside those gates SHALL be documented; they need not be suppressed.

#### Scenario: A clean configure and build emit no project warnings [vh_warning_free_build]

- **WHEN** the project is configured and built from a clean build directory
- **THEN** CMake and the C++ compiler print no warning caused by project code or
  configuration

#### Scenario: The local verification gates emit no project warnings [vh_warning_free_gates]

- **WHEN** `scripts/verify-fast.sh` and `scripts/verify-full.sh` run
- **THEN** their output contains no warning caused by project code or
  configuration
