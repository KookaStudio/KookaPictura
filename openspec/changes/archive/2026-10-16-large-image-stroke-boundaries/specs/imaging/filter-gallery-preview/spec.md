## ADDED Requirements

### Requirement: Filter Gallery previews a reduced copy

The Filter Gallery SHALL render its preview from a copy of the visible part of the document, plus a small margin, box-averaged down to at most one sample per device pixel of the preview pane. The copy is never scaled up. Each change to the stack SHALL re-filter only that copy. Preview cost therefore follows the pane's size and not the document's. While the gallery is open, neither the document nor the canvas SHALL change. OK SHALL apply the visible stack to the full-resolution layer as one "Filter Gallery" history state. Below 100% zoom the preview is an approximation: the filters run at the copy's resolution, so their pixel-sized parameters act larger relative to the picture than they do in the commit.

#### Scenario: A large picture previews at pane resolution

- **WHEN** the gallery opens on a 4000×3000 document in a pane narrower than 1000 device pixels
- **THEN** the preview image is narrower than 1000 device pixels and keeps the 4:3 aspect

#### Scenario: Previewing leaves the document alone

- **WHEN** the gallery previews a visible effect and is cancelled
- **THEN** the canvas is pixel-identical to before it opened and the history is unchanged

#### Scenario: Full-scale copy matches the commit

- **WHEN** the copy is taken at scale 1 over the whole document and filtered
- **THEN** it equals the composite of the document with the same filter applied to the layer
