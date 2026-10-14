# Spec Delta

## ADDED Requirements

### Requirement: In-app design-system browser

The application SHALL provide an in-app design-system browser reachable from
Help ▸ Developer ▸ Design System…, shown as a modeless tool window. The browser
SHALL present the design system using the application's own widgets and the
global stylesheet rather than a reimplementation, grouped into foundations,
shell, controls, and patterns, with a filterable story list, a per-story
description, and a token readout for the current brightness level. Every
primitive the global stylesheet defines SHALL appear on some story.

#### Scenario: The browser opens modelessly [ds_browser_modeless]

- **WHEN** the user chooses Help ▸ Developer ▸ Design System…
- **THEN** a modeless tool window opens on the grouped story list with a story
  shown, and the rest of the application remains interactive

#### Scenario: Stories render the real widgets [ds_browser_real_widgets]

- **WHEN** a story is selected
- **THEN** its page is built from the application's own widgets, styled by the
  global stylesheet, so it matches the running interface exactly

#### Scenario: Every primitive is covered [ds_browser_coverage]

- **WHEN** the global stylesheet declares a primitive
- **THEN** the browser renders an instance of it on some story

#### Scenario: Brightness preview is live and restored [ds_browser_brightness]

- **WHEN** the brightness control is changed
- **THEN** the application recolours to that level immediately; and when the
  browser is closed, the level in effect when it opened is restored, so
  previewing never changes the user's brightness preference

#### Scenario: The story list is filterable [ds_browser_filter]

- **WHEN** text is entered in the filter field
- **THEN** only the stories whose titles match remain visible

#### Scenario: The browser is covered by a Qt test [ds_test_browser]

- **WHEN** the design-system test opens the browser and builds every story
- **THEN** every story page builds without error, each curated primitive is
  present, the filter narrows the list, and closing restores the brightness
  level
