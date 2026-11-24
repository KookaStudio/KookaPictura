# Spec Delta

## MODIFIED Requirements

### Requirement: Additional adjustment-layer kinds

The New Adjustment Layer menu, the Layers panel New Fill / Adjustment menu, and
the adjustment-layer creation path SHALL support all sixteen CS6 adjustment
kinds: Brightness/Contrast, Levels, Curves, Exposure, Vibrance, Hue/Saturation,
Color Balance, Black & White, Photo Filter, Channel Mixer, Color Lookup, Invert,
Posterize, Threshold, Gradient Map, and Selective Color. Each creates a
non-destructive adjustment layer whose block carries that kind's declared
new-layer default and whose display name matches the menu entry. The Layers panel
creation menu SHALL additionally offer the Solid Color…, Gradient…, and Pattern…
fill layers. An unrecognised kind SHALL still be refused without adding a layer.

A new-layer default MAY differ from the kind's dialog default: the dialog opens
on neutral parameters, while a new layer may carry a visible change so that
adding the layer has an effect. Where they differ the catalogue SHALL declare
both, and the new layer SHALL use the new-layer default.

A Color Lookup layer's block SHALL name one of the built-in Color Lookup presets,
and a newly created Color Lookup layer SHALL name `None`, so the stored name
resolves to the `None` preset index, the same preset the dialog opens on.

#### Scenario: Every supported kind creates a named layer

- **WHEN** each of the sixteen supported adjustment kinds is requested
- **THEN** a layer carrying that kind's adjustment block is added with the matching display name

#### Scenario: Unknown kind is refused

- **WHEN** an unrecognised adjustment kind is requested
- **THEN** no layer is added and the request fails

#### Scenario: The new kinds open on neutral defaults

- **WHEN** a Levels, Curves, Exposure, Vibrance, or Black & White layer is created
- **THEN** its parameters are that kind's CS6 dialog defaults, so a Curves layer opens as the identity

#### Scenario: The panel creation menu offers every kind

- **WHEN** the Layers panel New Fill / Adjustment menu is opened
- **THEN** it lists the Solid Color…, Gradient…, and Pattern… fill entries and
  all sixteen adjustment kinds, for nineteen entries total, and choosing an
  adjustment kind adds that layer

#### Scenario: A new Color Lookup layer names its preset [adjust_layer_color_lookup_preset]

- **WHEN** a Color Lookup adjustment layer is created and its stored block is
  passed to `color_lookup_preset_index`
- **THEN** the index is the `None` preset's index, so the stored name resolves to
  a built-in preset instead of a name no preset matches
