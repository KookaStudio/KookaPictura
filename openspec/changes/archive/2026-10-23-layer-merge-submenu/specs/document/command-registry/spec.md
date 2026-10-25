# Spec Delta

## MODIFIED Requirements

### Requirement: Full documented top-level menu tree
The system SHALL present the ten standard CS6 top-level menus -- File, Edit,
Image, Layer, Type, Select, Filter, View, Window, Help -- in that order.
Submenus SHALL follow the documented CS6 ordering, except that the Layer menu
SHALL group Merge Layers, Merge Visible, Stamp Visible, Stamp Selected, Flatten
Image, and Merge Clipping Mask under a `Merge` submenu. Commands without a
handler SHALL appear disabled rather than be omitted.

#### Scenario: All top-level menus are present
- **WHEN** the frame is shown
- **THEN** the menu bar contains File, Edit, Image, Layer, Type, Select, Filter, View, Window, and Help in that order

#### Scenario: Unimplemented leaf is disabled
- **WHEN** a documented command has no registered handler
- **THEN** its menu item is visible and disabled

#### Scenario: Merge commands are grouped under Layer > Merge [cr_layer_merge_submenu]
- **WHEN** the Layer menu is opened
- **THEN** it contains one Merge submenu holding Merge Layers, Merge Visible, Stamp Visible, Stamp Selected, Flatten Image, and Merge Clipping Mask, and none of those six commands appears at the Layer top level
