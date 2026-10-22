## ADDED Requirements

### Requirement: The Warp Text button shows the Warp Text icon

The type tools' Warp Text buttons SHALL show the `options.warpText` icon, a T
standing on an arc, and not Free Transform's warped-grid icon.

#### Scenario: The button carries its own icon [wt_icon]

- **WHEN** `tst_warp_text::optionsButtonHasWarpTextIcon` inspects the
  Horizontal Type options page
- **THEN** the Warp Text button's icon is non-null and differs from the
  `options.warp` icon
