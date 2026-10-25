# Spec Delta

## ADDED Requirements

### Requirement: The engine layer-mode list is the only blend-mode table

The engine SHALL publish its layer blend-mode list as ordered (PSD key, CS6 display
name, has-neutral-colour) rows: the 27 layer modes, Normal through Luminosity,
with Pass Through excluded. Every layer blend-mode control and label in the shell SHALL build from
that list. That covers the New
Layer dialog, the Layer Style blend combo, the Layers panel blend combo and Mode
filter, and the Properties panel's mode label. A control's selected index SHALL
be the engine list index. The shell SHALL hold no blend key, name or
neutral-colour table of its own, and the New Layer dialog's neutral-colour option
SHALL follow the engine's has-neutral flag.

#### Scenario: Engine list order and names [blend_list_engine]

- **WHEN** the engine layer-mode list is enumerated
- **THEN** it has 27 entries in layer-mode order, with each key round-tripping
  through the PSD key decoder and each name equal to the mode's display name

#### Scenario: Layer combos build from the engine list [blend_list_combos]

- **WHEN** the Layers panel blend combo and Mode filter, the New Layer dialog's
  Mode combo and the Layer Style blend combo are populated
- **THEN** each lists the engine names in engine order. The Layers panel, filter
  and New Layer combos carry the engine keys as item data, and the Layer Style
  combo stays index-only, its index being the engine index.

#### Scenario: Choosing a mode sets that engine mode [blend_list_set]

- **WHEN** the user picks the entry at index *i* in the Layers panel blend combo
- **THEN** the active layer's blend key is the engine list's key at index *i*
