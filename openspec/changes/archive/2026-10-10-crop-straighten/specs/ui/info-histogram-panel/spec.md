# Spec Delta

## ADDED Requirements

### Requirement: Info panel tool hints

The Info panel's tool hint SHALL render each hint on its own line, with the first
letter of each hint capitalised, at a font two pixels smaller than the panel's
base font.

#### Scenario: Hints render one per line [uiih_hints_lines]

- **WHEN** a tool with several hints is active
- **THEN** each hint appears on its own line, capitalised, at the reduced font size

### Requirement: Info panel icon menus

An Info panel icon menu SHALL open fully laid out on its first show, positioned
beside its button and fully on screen, without clipping its items.

#### Scenario: First open is not clipped [uiih_menu_first_open]

- **WHEN** an icon menu button is opened for the first time
- **THEN** the whole menu is shown, laid out and unclipped

### Requirement: Histogram All Channels

The Histogram panel SHALL offer an `All Channels` entry, selected by default,
that draws the red, green, and blue channel curves as an overlaid composite.

#### Scenario: All Channels is the default [uiih_histogram_all_channels]

- **WHEN** the Histogram panel is shown
- **THEN** `All Channels` is selected and the red, green, and blue curves are drawn together

#### Scenario: A single channel can be chosen [uiih_histogram_single]

- **WHEN** a single channel is chosen
- **THEN** only that channel's curve is drawn
