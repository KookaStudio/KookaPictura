## ADDED Requirements

### Requirement: Group Blending Options writes

`set_layer_style_value(layer, key, value)` SHALL accept the `blending.*` keys for
a group and update the layer record. It SHALL continue to refuse effect keys on a
group until the compositor renders group effects.

#### Scenario: A group's opacity is written

- **WHEN** `blending.opacity` is set to 50 on a group
- **THEN** the group's opacity is 128 and `dropShadow.on` remains refused
