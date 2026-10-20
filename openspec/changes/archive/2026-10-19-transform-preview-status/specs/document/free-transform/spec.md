## MODIFIED Requirements

### Requirement: Live preview reuses the move-preview overlay

The session SHALL preview by drawing a cached base composite with the target
layer hidden plus a cached image of the target layer under the current
`QTransform`, reusing the Move tool's cached base/layer overlay. The session MUST
NOT rebuild the document composite on each mouse move. For a channel-less
embedded target the preview layer image SHALL be rendered from its embedded
source without mutating the document. When the target's raw channels do not
carry its whole appearance — a layer mask, vector mask, layer style, fill
adjustment, clipping group, or editable type — the preview layer image SHALL instead be rendered
through the compositor with the layer's opacity and blend neutralized, cropped
to the layer's rect, so the live preview keeps that appearance. The preview
SHALL apply the layer opacity once.

#### Scenario: A drag does not recomposite

- **WHEN** a session gesture updates the transform
- **THEN** the cached base and layer images are redrawn with the new transform and no document composite is rebuilt

#### Scenario: Previewing a channel-less target mutates nothing

- **WHEN** a session is begun on a channel-less embedded smart object
- **THEN** the document equals its pre-session value until commit

#### Scenario: A styled layer keeps its color [ft_preview_appearance]

- **WHEN** a transform session begins on a layer whose visible color comes from a layer style, layer mask, vector mask, fill adjustment, clipping group, or type
- **THEN** the live preview shows that appearance rather than the bare channels
