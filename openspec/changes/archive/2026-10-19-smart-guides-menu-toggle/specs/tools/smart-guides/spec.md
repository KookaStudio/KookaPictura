## ADDED Requirements

### Requirement: Smart Guides visibility toggle

`View > Show > Smart Guides` SHALL be a checkable command, on by default, that
controls whether smart guides are active. Turning it off SHALL clear any active
guides and SHALL stop Move and Type drags from snapping or drawing guides;
turning it on SHALL restore both. The setting SHALL apply to every open document
and SHALL persist across restarts.

#### Scenario: The menu turns smart guides off and on [sg_toggle]

- **WHEN** `tst_smart_guides::smartGuidesToggle` triggers `View > Show > Smart Guides` off, drags a layer near the document centre, then triggers it on
- **THEN** no guide is drawn and the layer keeps the pointer offset while off; a guide is drawn and the layer snaps while on; and the session stores the choice
