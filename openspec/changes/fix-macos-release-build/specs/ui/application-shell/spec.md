# Spec Delta

## ADDED Requirements

### Requirement: Menu bar uses the native macOS menu bar

On macOS the application shell SHALL present its menus through the native
system-wide menu bar rather than drawing a menu bar inside the window, and SHALL
NOT disable native menu-bar use. On platforms without a native menu bar the
shell SHALL draw the menu bar inside the window. Entering a screen mode SHALL NOT
change which of these two presentations is in effect.

#### Scenario: The native bar is used on macOS

- **WHEN** the frame is shown on macOS
- **THEN** the window's menu bar reports that it is the native menu bar and no
  menu bar is drawn inside the window frame

#### Scenario: The in-window bar is used elsewhere

- **WHEN** the frame is shown on a platform without a native menu bar
- **THEN** the window's menu bar is drawn inside the window frame

#### Scenario: Screen modes keep the platform presentation

- **WHEN** the frame enters or leaves Full Screen With Menu Bar and Full Screen
  modes on macOS
- **THEN** the menu bar continues to be presented by the system, and the shell
  does not switch to an in-window menu bar

#### Scenario: Application-menu roles are honoured on macOS

- **WHEN** the menus containing the About, Preferences, and Quit commands are
  built on macOS
- **THEN** those entries are eligible for placement in the macOS Application
  Menu through the platform's menu-role merging
