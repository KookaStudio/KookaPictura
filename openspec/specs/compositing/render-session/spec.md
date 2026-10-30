# compositing/render-session Specification

## Purpose
Presents a live edit preview from the smallest frame the canvas shows — the
visible document region at the presented view-pyramid level, reusing the
composite below the edited layer — and replaces it with an exact full composite
when the edit commits.

## Requirements

### Requirement: A preview composites the visible region at the presented level

For a live preview of an edit whose effect is the whole canvas — an
adjustment-layer parameter, curve, or reset, or a canvas-sized opacity, blend,
fill, or visibility change — the application SHALL composite only the visible
document rectangle at the view-pyramid level the canvas presents. It MUST NOT
composite pixels outside the visible document rectangle at full resolution to
present a preview.

At level 0 the preview SHALL equal the exact full composite over that rectangle.
At a level above 0 the preview MAY differ from the corresponding level of the
exact full composite; the commit SHALL replace it with the exact frame.

#### Scenario: A zoomed-out preview composites only the visible region at the presented level [rs_preview_region]

- **WHEN** a canvas-sized adjustment parameter is previewed while the canvas presents a level above 0
- **THEN** the preview composites the visible document rectangle at that level and no pixels outside it are composited at full resolution

#### Scenario: A level-0 preview is exact in the visible region [rs_preview_level0]

- **WHEN** the same edit is previewed while the canvas presents level 0
- **THEN** the presented rectangle equals the corresponding rectangle of the exact full composite

#### Scenario: A reduced preview may be approximate [rs_preview_approximate]

- **WHEN** a preview is presented at a level above 0
- **THEN** the presented frame need not equal that level of the exact full composite, and the commit replaces it with the exact frame

### Requirement: A preview is display-only and the commit is exact

A preview SHALL be presented into the display levels only. It MUST NOT update
the document's maintained composite or any display level outside the presented
rectangle. At level 0 it MAY patch the presented rectangle of the level-0 display
frame. The commit that finishes the edit SHALL produce the exact full composite,
replace the approximate frame, and capture the exact state as one history state.

#### Scenario: The maintained composite is unchanged during a preview [rs_preview_display_only]

- **WHEN** a preview has been presented and the maintained composite is read before the commit
- **THEN** it equals its value before the gesture and the display levels outside the presented rectangle equal their values before the gesture

#### Scenario: The commit produces the exact frame [rs_commit_exact]

- **WHEN** the previewed edit commits
- **THEN** the document's composite equals a full composite of the committed stack, every display level equals a rebuild from it, and exactly one history state is recorded

#### Scenario: A preview that changes nothing commits nothing [rs_commit_noop]

- **WHEN** the final previewed value equals the value before the gesture
- **THEN** the commit records no history state and the composite is unchanged

### Requirement: The composite below the edited layer is reused as a backdrop

The application SHALL cache the composite of the layers below the edited layer
at the preview's resolution and reuse it while that layer prefix is unchanged.
At level 0, compositing the cached backdrop and the layers at and above the
edited layer SHALL equal the ordinary composite of the same region. Above level
0 it SHALL equal the reduced composite at that level whenever the resume is
valid.

#### Scenario: Backdrop plus suffix equals the full composite [rs_backdrop_exact]

- **WHEN** the edited layer is a root-level layer with a valid resume and the preview is at level 0 over the whole document
- **THEN** the backdrop plus the layers at and above it equals the full composite byte for byte, including blend modes, clipping, and knockout

#### Scenario: A second frame reuses the backdrop [rs_backdrop_reuse]

- **WHEN** a second preview frame runs while the layer prefix is unchanged
- **THEN** the prefix is not composited again and the frame equals the same composite

### Requirement: The backdrop invalidates on any change below the edited layer

The cached backdrop SHALL be invalidated when any input that can change its
pixels changes: a pixel edit, a property edit, a reorder, a structural edit, a
visibility change, or a document size or mode change. A stale backdrop MUST NOT
be served. An edit at or above the edited layer MUST NOT invalidate the backdrop.

#### Scenario: A pixel edit below the layer invalidates [rs_invalidate_pixel]

- **WHEN** a layer below the edited layer changes its pixels
- **THEN** the next preview rebuilds the backdrop and the result equals a full composite

#### Scenario: A property edit below the layer invalidates [rs_invalidate_property]

- **WHEN** opacity, blend mode, fill, or visibility changes below the edited layer
- **THEN** the next preview rebuilds the backdrop and the result equals a full composite

#### Scenario: A reorder invalidates [rs_invalidate_reorder]

- **WHEN** layers are reordered
- **THEN** the backdrop is rebuilt before the next preview

#### Scenario: An edit at or above the layer keeps the backdrop [rs_invalidate_above]

- **WHEN** only the edited layer or a layer above it changes
- **THEN** the cached backdrop is still valid and the preview reuses it

### Requirement: Only a structurally valid resume is used

The application SHALL resume from a cached backdrop only when the resume
boundary is structurally valid. It MUST fall back to the ordinary full composite
when the edited layer is inside a clipping run, inside a group, when a knockout
layer above the resume point cannot be given its base, when the document is not
RGB, when it retains native depth, or while a stroke is live.

#### Scenario: A clipped target falls back [rs_resume_clipping]

- **WHEN** the edited layer is clipped to a layer below it
- **THEN** the preview and the commit use the full composite and equal it

#### Scenario: A nested target falls back [rs_resume_nested]

- **WHEN** the edited layer is inside a group
- **THEN** the preview and the commit use the full composite and equal it

#### Scenario: A knockout above the resume point is exact [rs_resume_knockout]

- **WHEN** a layer above the edited layer knocks out
- **THEN** the result equals the full composite, with the knockout base present or the full path taken

#### Scenario: A valid root-level target resumes [rs_resume_valid]

- **WHEN** the edited layer is a non-clipped root-level layer and the document is RGB at 8-bit
- **THEN** the resumed composite equals the full composite

### Requirement: Preview requests coalesce per presented frame

The application SHALL run at most one preview composite per presented frame,
however many parameter changes arrive in that frame, and SHALL present the
latest value. The commit SHALL reflect the latest value, not an intermediate
one.

#### Scenario: Many changes produce one frame [rs_coalesce]

- **WHEN** several parameter changes arrive between two presented frames
- **THEN** at most one preview composite runs and the frame shows the latest value

#### Scenario: The commit uses the latest value [rs_coalesce_commit]

- **WHEN** the edit commits after coalesced previews
- **THEN** the committed state and the history state reflect the latest value

### Requirement: An unavailable preview falls back without lying

When a preview cannot be served — no stored level to present, no presented
level, an unsummarisable layer prefix, a soloed mask, a non-RGB or native-depth
document, a live stroke, content that cannot be reduced (a live type layer, a
proxy-less smart object, or a vector fill), or a backdrop plus scratch that would
exceed the session's memory budget — the application SHALL use the existing exact
path. An empty visible rectangle SHALL present nothing and mark no damage. It
MUST NOT present an approximate frame as the exact composite.

#### Scenario: No stored level falls back [rs_fallback_no_level]

- **WHEN** the document's pyramid has no level above 0 and a canvas-sized edit is previewed
- **THEN** the exact path runs and the presented frame equals the full composite

#### Scenario: An oversized backdrop is refused [rs_fallback_budget]

- **WHEN** the backdrop plus the scratch canvas would exceed the session's memory budget
- **THEN** no backdrop is cached and the exact path runs

#### Scenario: A soloed mask or live stroke falls back [rs_fallback_state]

- **WHEN** a mask is soloed or a stroke is live
- **THEN** the exact path runs and the displayed frame is the one that state defines

#### Scenario: Content that cannot be reduced falls back [rs_fallback_content]

- **WHEN** the edited stack contains a live type layer, a proxy-less smart object, or a vector fill
- **THEN** the preview and the commit use the exact path and equal the full composite

#### Scenario: An empty visible rectangle is a no-op [rs_fallback_empty]

- **WHEN** the visible document rectangle is empty
- **THEN** no preview is presented and no damage is marked

#### Scenario: No presented level falls back [rs_fallback_no_presented_level]

- **WHEN** the canvas reports no presented level
- **THEN** the exact path runs and the frame equals the full composite
