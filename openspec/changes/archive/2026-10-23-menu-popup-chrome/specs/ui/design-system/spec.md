# Spec Delta

## ADDED Requirements

### Requirement: Menu popup chrome
Menu popups SHALL paint an outline with a corner radius of at least 3 CSS pixels
at 100 percent UI scale and SHALL be translucent outside that outline. Their
items SHALL be inset from the outline by 2 CSS pixels at 100 percent scale. An
item that carries an icon or check mark SHALL leave at least 8 CSS pixels
between that glyph and the popup's left edge at 100 percent scale.

#### Scenario: Rounded popup corners [ds_menu_rounded]
- **WHEN** a menu is shown
- **THEN** the popup is transparent outside the rounded outline and a selected top row does not paint across the corner

#### Scenario: Icon inset [ds_menu_icon_padding]
- **WHEN** a menu item carries an icon or check mark
- **THEN** the glyph's left edge sits at least 8 CSS pixels from the popup's left edge at 100 percent scale
