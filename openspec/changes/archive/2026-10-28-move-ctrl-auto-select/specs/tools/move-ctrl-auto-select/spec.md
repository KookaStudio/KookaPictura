## ADDED Requirements

### Requirement: Ctrl temporarily inverts Move Auto-Select

With the Move tool and no active selection, a press with Ctrl held SHALL apply the inverse of the stored Auto-Select option: when Auto-Select is off, the topmost visible layer with a pixel under the pointer SHALL become active, resolved to its top-level group when the Auto-Select dropdown is Group; when Auto-Select is on, the active layer SHALL be kept. While Ctrl is held, the options bar SHALL show the Auto-Select checkbox in its flipped state, with the dropdown enabled to match, and SHALL restore the stored state when Ctrl is released; holding Ctrl SHALL NOT change the stored option.

#### Scenario: Ctrl-press with Auto-Select off selects the layer under the pointer [mca_ctrl_click_selects]

- **WHEN** `tst_options_bars_parity::moveCtrlClickAutoSelects` sets Auto-Select off with the Layer target, makes a transparent upper layer active, and Ctrl-presses on an opaque pixel of a lower layer under a transparent area of the upper one
- **THEN** the lower layer becomes active

#### Scenario: Ctrl-press with Auto-Select on keeps the active layer [mca_ctrl_disables]

- **WHEN** `tst_options_bars_parity::moveCtrlClickSuppressesAutoSelect` sets Auto-Select on and Ctrl-presses on another layer's pixels
- **THEN** the active layer is unchanged

#### Scenario: Held Ctrl shows the flipped checkbox [mca_checkbox_shows_flip]

- **WHEN** `tst_options_bars_parity::moveCtrlShowsFlippedAutoSelect` holds Ctrl on the canvas with Auto-Select off, then releases it
- **THEN** the checkbox shows checked and the dropdown enabled while Ctrl is held, both return to unchecked and disabled on release, and the stored option stays off throughout

#### Scenario: Ctrl-press follows the Group target [mca_ctrl_follows_mode]

- **WHEN** `tst_options_bars_parity::moveCtrlClickFollowsGroupMode` sets Auto-Select off with the Group target and Ctrl-presses on a pixel of a layer inside a group
- **THEN** the top-level group becomes active
