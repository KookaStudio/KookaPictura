## ADDED Requirements

### Requirement: Selective Color composite channel

The Image adjustment command and the adjustment-layer creation launchers SHALL offer Selective Color only when their existing prerequisites hold and the active document's composite channel is selected and displayed. This MUST apply to the Image and Layer menus, the Adjustments panel tile and flyout, and the Layers footer menu. Invoking an unavailable launcher MUST leave the document and history unchanged.

#### Scenario: Selecting a component channel disables all launchers [selective_color_component]

- **WHEN** a user selects Red in an RGB document
- **THEN** the Selective Color launchers are disabled, an attempted dispatch opens no dialog or layer, and another otherwise eligible adjustment stays available
- **WHEN** the user restores the RGB composite
- **THEN** the launchers are enabled again

#### Scenario: Channel visibility and masks affect availability [selective_color_display]

- **WHEN** the working composite is partly hidden, an alpha is displayed, a layer mask is soloed, or Quick Mask is active
- **THEN** Selective Color is unavailable
- **WHEN** the user returns to the normal composite
- **THEN** Selective Color is available again

#### Scenario: External mask solo changes availability [selective_color_external_mask_solo]

- **WHEN** a user solos the active layer mask with Alt-click or Ctrl+\ while its Channels row is not selected
- **THEN** the Selective Color launchers become unavailable
- **WHEN** the user ends the solo using the same control
- **THEN** the launchers again reflect the selected and displayed channel state

#### Scenario: Availability follows the active document [selective_color_document]

- **GIVEN** two RGB documents and a component channel selected in the active document
- **WHEN** the user switches to the other document and Channels resets to its composite
- **THEN** the other document's composite permits Selective Color

#### Scenario: Multichannel has no composite [selective_color_multichannel]

- **WHEN** the active document is Multichannel
- **THEN** Selective Color is unavailable because that mode has no composite row
