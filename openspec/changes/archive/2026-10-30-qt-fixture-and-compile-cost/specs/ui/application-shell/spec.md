## ADDED Requirements

### Requirement: Building the main window applies the theme once

Constructing the main window SHALL apply the interface theme for the restored
family and brightness level after the restored UI scale and the application
font are in effect, and before the window creates its menus, panels and tool
controls, so those widgets are built under the final style rather than
re-polished by a later theme change. Applying
the theme SHALL leave the application untouched when the application's current
style is the theme's style and its current palette and style sheet already
equal the ones the theme would install for the requested family, brightness
level and UI scale. The comparison SHALL be made against the application's
current state, so a palette or style sheet changed outside the theme is
replaced by the next theme application. A change of family, brightness level
or UI scale SHALL still apply the theme immediately. After construction, the
application palette and style sheet SHALL equal the ones a theme application
with the restored settings produces, and the fonts the window sets explicitly
on its document tab bar and status bar while it is built SHALL still be in
effect.

#### Scenario: a second window does not re-apply an unchanged theme [las_theme_idempotent]

- **WHEN** a second main window is constructed in the same process with the same restored family, brightness level and UI scale
- **THEN** the application style object, palette and style sheet are not replaced and the icon cache generation does not advance

#### Scenario: a brightness change still applies [las_theme_change_applies]

- **WHEN** the brightness level of a shown window is stepped from a level below the highest
- **THEN** the theme is applied with the new level, the icon cache generation advances, and the application palette changes

#### Scenario: the chrome is built under the restored theme [las_theme_before_chrome]

- **WHEN** the main window is constructed with a restored light family and a non-default brightness level
- **THEN** the application style sheet is already the restored theme's when the panels and the tool options bar build their contents, and the finished application palette and style sheet equal the ones a theme application with those settings produces

#### Scenario: an outside change does not defeat the next application [las_theme_live_state]

- **WHEN** the theme is applied, the application style sheet is then cleared outside the theme, and the same theme is applied again
- **THEN** the application style sheet equals the theme's style sheet again

#### Scenario: explicit chrome fonts survive construction [las_theme_chrome_fonts]

- **WHEN** the main window has been constructed
- **THEN** the document tab bar and the status bar carry the fonts the window set on them when it built them
