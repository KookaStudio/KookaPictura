# Spec Delta

## ADDED Requirements

### Requirement: Blend-mode availability follows the document mode and depth

The engine SHALL decide which of the 27 layer blend modes a document offers from
the document's effective colour mode and bit depth, and every layer blend-mode
authoring control SHALL offer only the available modes. The effective mode and
depth SHALL be the same values the Channels panel reports, so a document whose
working pixels are RGB but whose file mode is Lab is gated as Lab.

- A 32-bit document SHALL offer exactly these 15 modes: Normal, Dissolve,
  Darken, Multiply, Lighten, Darker Color, Linear Dodge (Add), Lighter Color,
  Difference, Subtract, Divide, Hue, Saturation, Color, and Luminosity. The
  other 12 modes SHALL NOT be selectable.
- A Lab document SHALL offer all modes except Color Dodge, Color Burn, Darken,
  Lighten, Difference, Exclusion, Subtract, and Divide.
- A Bitmap or Indexed document SHALL offer Normal only. A Bitmap document MAY
  label that entry `Threshold`.
- An 8- or 16-bit RGB, Grayscale, or CMYK document SHALL offer all 27 modes.
- The Layers panel blend combo, the New Layer / New Group dialog's Mode combo,
  the Layer Style dialog's Blending Options mode and every effect mode, and the
  Fade dialog's Mode menu SHALL all use the same availability result. No control
  SHALL keep an independent availability table.
- A layer whose stored mode is unavailable in the current document SHALL keep
  that mode: the control SHALL show it selected and SHALL NOT let the user
  choose it, rather than silently presenting Normal while the layer still
  composites the stored mode.
- Changing the document's mode or depth SHALL update every open control's
  offered modes, and converting back to a document that allows a mode SHALL
  restore the full list.

#### Scenario: A 32-bit document offers 15 modes [bma_hdr]

- **WHEN** a 32-bit RGB document is active
- **THEN** each layer blend-mode control offers Normal, Dissolve, Darken,
  Multiply, Lighten, Darker Color, Linear Dodge (Add), Lighter Color,
  Difference, Subtract, Divide, Hue, Saturation, Color, and Luminosity as
  selectable entries and offers no other mode for selection; an unavailable
  stored mode is shown selected and disabled, as in [bma_conflict]

#### Scenario: A Lab document excludes 8 modes [bma_lab]

- **WHEN** a Lab document is active
- **THEN** each layer blend-mode control offers all modes except Color Dodge,
  Color Burn, Darken, Lighten, Difference, Exclusion, Subtract, and Divide as
  selectable entries; an unavailable stored mode is shown selected and disabled,
  as in [bma_conflict]

#### Scenario: Bitmap and Indexed offer Normal only [bma_bitmap]

- **WHEN** a Bitmap or Indexed document is active
- **THEN** the availability result offers Normal only, and a populated layer
  blend-mode control offers no other selectable mode

#### Scenario: Every layer blend-mode control follows the gate [bma_controls]

- **WHEN** a document whose mode or depth restricts the menu is active
- **THEN** the Layers panel combo, the New Layer dialog, the Layer Style layer
  and effect combos each offer exactly the available modes for selection, with
  an unavailable stored or default mode present but disabled, and the Fade mode
  menu presents every excluded mode disabled

#### Scenario: An unavailable stored or default mode is shown, not replaced [bma_conflict]

- **WHEN** a layer or effect holds a mode the current document does not allow,
  or an absent effect's default mode is unavailable
- **THEN** its blend-mode control shows that mode's name selected and disabled,
  and the layer's or effect's mode is unchanged until the user chooses an
  available mode.

#### Scenario: Converting back restores the full list [bma_restore]

- **WHEN** a 32-bit or Lab document is converted to 8-bit RGB
- **THEN** each layer blend-mode control offers all 27 modes again and a layer's
  stored mode becomes selectable

## MODIFIED Requirements

### Requirement: The engine layer-mode list is the only blend-mode table

The engine SHALL publish its layer blend-mode list as ordered (PSD key, CS6 display
name, has-neutral-colour) rows: the 27 layer modes, Normal through Luminosity,
with Pass Through excluded. Every layer blend-mode control and label in the shell
SHALL build from that list. That covers the New Layer dialog, the Layer Style
blend combo, the Layers panel blend combo and Mode filter, and the Properties
panel's mode label. A control's selected entry SHALL carry the engine key as item
data for the Layers panel, the Mode filter and the New Layer combo, and the engine
index as item data for the Layer Style combo; its display index is the entry's
position in the offered (filtered) list, which is the engine order but not
necessarily the engine index. The shell SHALL hold no blend key, name or
neutral-colour table of its own, and the New Layer dialog's neutral-colour option
SHALL follow the engine's has-neutral flag.

#### Scenario: Engine list order and names [blend_list_engine]

- **WHEN** the engine layer-mode list is enumerated
- **THEN** it has 27 entries in layer-mode order, with each key round-tripping
  through the PSD key decoder and each name equal to the mode's display name

#### Scenario: Layer combos build from the engine list [blend_list_combos]

- **WHEN** the Layers panel blend combo and Mode filter, the New Layer dialog's
  Mode combo and the Layer Style blend combo are populated
- **THEN** each lists the offered engine names in engine order. The Layers panel,
  filter and New Layer combos carry the engine keys as item data, and the Layer
  Style combo carries the engine index as item data; the display index is the
  entry's position in the offered list.

#### Scenario: Choosing a mode sets that engine mode [blend_list_set]

- **WHEN** the user picks the entry at display index *i* in the Layers panel
  blend combo
- **THEN** the active layer's blend key is the engine key stored in that entry's
  item data
