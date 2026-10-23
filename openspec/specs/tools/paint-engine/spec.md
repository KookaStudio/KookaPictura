# paint-engine Specification

## Purpose
The brush tip engine: coverage, spacing, flow and opacity accumulation, pencil aliasing, paint modes, and per-stroke undo.

## Requirements

### Requirement: Standard brush tip coverage

The system SHALL render a procedural round/elliptical tip whose coverage is a
radial profile: fully opaque inside `hardness%` of the tip radius, falling off
smoothly to zero at the radius. Size SHALL be the tip diameter in pixels,
roundness SHALL squash the short axis, and angle SHALL rotate the profile. The
Brush SHALL keep a minimum anti-aliasing ramp so that hardness 100% still
produces a soft edge; coverage values SHALL be accumulated per pixel over a
stroke.

#### Scenario: Dabs are stamped along a stroke

- **WHEN** a stroke is painted with size `d` and fixed spacing `s%`
- **THEN** consecutive dab centers are spaced `s% × d` apart along the path

#### Scenario: Brush edge is anti-aliased at full hardness

- **WHEN** the Brush paints with hardness 100% on a hard-edged test document
- **THEN** at least one pixel at the stroke boundary has partial coverage, not only full or zero

#### Scenario: Elliptical tip

- **WHEN** roundness is below 100%
- **THEN** the painted footprint is narrower across the short axis than across the long axis

### Requirement: Stroke spacing

The system SHALL place dabs at a fixed arc-length interval when spacing is a
percentage of the diameter, and SHALL place dabs according to pointer speed when
spacing is disabled. Resampling SHALL carry the residual distance across
samples so spacing stays uniform on fast strokes, and SHALL clamp to a minimum
geometric step so coincident samples do not multiply dabs without bound.

#### Scenario: Increasing spacing separates dabs

- **WHEN** the same path is painted at a small spacing and then at a large spacing
- **THEN** the large-spacing stroke leaves visibly separated marks

#### Scenario: Coincident samples do not explode the dab count

- **WHEN** many samples arrive at the same position with a very low spacing
- **THEN** the number of placed dabs stays bounded and painting does not stall

### Requirement: Flow and opacity accumulation

The system SHALL treat opacity as a per-stroke ceiling on the paint applied to
any pixel and flow as the rate at which coverage builds toward that ceiling.
Repeated passes within one held stroke SHALL NOT exceed the opacity value, and
releasing and painting a new stroke SHALL apply an additional independent step
of colour.

#### Scenario: Opacity caps a single stroke

- **WHEN** a stroke with opacity `O` backtracks over the same pixels repeatedly before release
- **THEN** the resulting coverage at those pixels does not exceed `O` plus rounding

#### Scenario: A second stroke adds coverage

- **WHEN** a pixel is painted by one stroke and then by a second stroke with the same settings
- **THEN** the coverage after the second stroke is greater than after the first and does not exceed the opacity ceiling

#### Scenario: Flow is the build-up rate

- **WHEN** two strokes use the same opacity but different flow values
- **THEN** the lower-flow stroke reaches less coverage in a single pass than the higher-flow stroke

### Requirement: Pencil aliased edge and Auto Erase

The system SHALL paint the Pencil tool with a hard, non-anti-aliased edge, so
every covered pixel is fully painted and boundary pixels are either fully
painted or untouched. When Auto Erase is enabled and the stroke begins over a
pixel matching the foreground colour, the Pencil SHALL erase toward the
background colour or transparency instead of painting.

#### Scenario: Pencil edge is aliased

- **WHEN** the Pencil paints a stroke at any hardness
- **THEN** no boundary pixel has partial coverage

#### Scenario: Auto Erase begins over the foreground colour

- **WHEN** Auto Erase is on and the Pencil starts a stroke on a pixel equal to the foreground colour
- **THEN** that stroke removes colour instead of adding the foreground colour

### Requirement: Paint modes

The system SHALL support the paint modes `Normal`, `Dissolve`, `Behind`,
`Clear`, and `Multiply`. `Normal` interpolates the destination toward the
paint colour by the composited coverage; `Dissolve` selects the paint colour
stochastically per pixel in proportion to coverage; `Behind` writes only where
the destination is not fully opaque; `Clear` reduces the destination alpha by
the coverage; `Multiply` interpolates the destination toward the product of
the paint colour and the destination colour by the composited coverage, and
paints the plain colour where the destination is transparent.

#### Scenario: Normal blends by coverage

- **WHEN** a Normal stroke paints at full opacity over an opaque base
- **THEN** every fully covered pixel becomes the paint colour

#### Scenario: Clear reduces alpha

- **WHEN** a Clear stroke paints at full coverage on an opaque pixel
- **THEN** that pixel's alpha becomes zero

#### Scenario: Behind leaves opaque pixels untouched

- **WHEN** a Behind stroke paints over an opaque pixel
- **THEN** that pixel's colour is unchanged

#### Scenario: Multiply darkens by the backdrop

- **WHEN** a Multiply stroke paints at full opacity over an opaque base and
  over a transparent pixel
- **THEN** each opaque pixel becomes the channel-wise product of the paint
  colour and the base, and the transparent pixel takes the plain paint colour

### Requirement: Per-stroke commit and undo

The system SHALL commit a completed stroke as exactly one undoable history state
and SHALL make undo restore every touched pixel bit-exactly. A stroke that
painted no pixel SHALL add no history state and SHALL NOT mark the document
dirty. Undo requested while a stroke is in progress SHALL be ignored or cancel
the stroke without committing it.

#### Scenario: One history state per stroke

- **WHEN** a stroke is completed
- **THEN** the history gains exactly one labelled state and the document is dirty

#### Scenario: Undo restores the pixels

- **WHEN** a completed stroke is undone
- **THEN** every pixel the stroke touched returns to its pre-stroke value

#### Scenario: Empty stroke is a no-op

- **WHEN** a stroke completes without covering any pixel
- **THEN** no history state is added and the document is not marked dirty

#### Scenario: Undo mid-stroke is ignored

- **WHEN** undo is requested while a paint stroke is still in progress
- **THEN** the in-progress stroke is not committed by the undo

### Requirement: Incremental dirty region and dab latency

The paint engine SHALL report an incremental dirty rectangle rather than the
cumulative dirty rectangle of the whole stroke, so compositing after a dab
touches only the pixels that dab changed. The rectangle a present carries SHALL
cover the footprints of the dabs placed since the previous present and MUST NOT
grow with the dabs of earlier presents in the same stroke. The CPU region
compositor SHALL composite only the requested region rather than fully
compositing the document and slicing the result, and a region blit SHALL update
only that region of the presented image instead of invalidating the whole scaled
present cache. The sustained cost of a brush dab on a 4000×4000 document SHALL
meet the canvas-view budget (`docs/dev/canvas-view-spec.md`): input-to-first-pixel
at most 16 ms per dab and a sustained redraw at or above 60 FPS.

#### Scenario: A dab dirties only its own region [lpe_dirty_per_dab]

- **WHEN** a stroke places a dab after other dabs
- **THEN** the reported dirty rectangle covers the dabs since the previous
  present, not the union of all prior dabs of the stroke

#### Scenario: A present covers only the dabs since the previous present
[lpe_dirty_since_last_present]

- **WHEN** several dabs are placed between two presents of the same stroke
- **THEN** the reported dirty rectangle is no larger than the union of those
  dabs' footprints

#### Scenario: Region compositing matches the full composite [lpe_region_equal]

- **WHEN** a region is composited through the CPU region compositor
- **THEN** every pixel in that region equals the full-document composite within
  the compositor tolerance

#### Scenario: A dab meets the canvas-view budget [lpe_dab_budget]

- **WHEN** a brush dab is processed on a 4000×4000 document
- **THEN** the input-to-first-pixel latency is at most 16 ms and the sustained
  update rate is at least 60 FPS, measured against the canvas-view budget

### Requirement: Live paint stroke stays interactive on large documents

The live paint stroke SHALL stay interactive on large documents. While a paint
stroke is in progress the app SHALL update only the affected region and SHALL
keep the per-dab work bounded, free of whole-document costs: the command
registry, window/tab titles, and cursor policy SHALL NOT be recomputed on every
dab (they run once when the stroke commits), and the full-resolution canvas SHALL
NOT be detached or fully rebuilt per dab. The in-stroke region composite SHALL
use whichever backend is faster for the document (measured on the reference
machine: the GPU region composite is faster than the CPU oracle for a 512 px
brush dab). Releasing the mouse SHALL still commit exactly one history state and
SHALL refresh only the stroke's changed rectangle (its composite matches a full
recomposite) while the panels, registry, and titles refresh once. Cancelling a
stroke SHALL likewise restore the changed rectangle from the unchanged document
without a full-document recomposite.

#### Scenario: Per-dab GUI work is coalesced [pe_dab_no_registry_refresh]

- **WHEN** many dabs are applied during one stroke
- **THEN** the command registry and window/tab titles are not refreshed per dab,
  and exactly one history state is recorded on release

#### Scenario: The commit runs the full refresh [pe_commit_refresh]

- **WHEN** the stroke is released
- **THEN** the stroke's changed rectangle is refreshed so the composite matches a
  full recomposite, no full-document composite runs, the panels/registry/titles
  refresh once, and exactly one history state exists

### Requirement: Reduced-resolution preview for large dabs

When a dab's bounding box would push its rasterization past the
input-to-first-pixel budget, the system SHALL present a reduced-resolution
preview of the in-progress stroke instead of rasterizing it at full resolution.
The preview SHALL cover the stroke's bounding box at a view-pyramid level whose
own rasterization and present fit the same budget as the full-resolution
rasterization it replaces, SHALL be built from a snapshot of that level taken at
stroke start rather than from a fresh scan of the document, and MUST NOT write
level 0. Dabs whose bounding box fits the budget SHALL rasterize at full
resolution exactly as before.

#### Scenario: A small dab is not previewed [pe_preview_below_threshold]

- **WHEN** a stroke runs with a dab bounding box at or under the raster budget
- **THEN** it rasterizes and presents at full resolution, with no reduced
  document and no change to the pixels it produces

#### Scenario: A large dab is previewed [pe_preview_above_threshold]

- **WHEN** a stroke runs with a dab bounding box above the raster budget
- **THEN** the in-progress stroke is presented at a view-pyramid level whose own
  work fits the budget, built from a snapshot taken at stroke start rather than
  a scan of the document, and the number of pixels rasterized per dab no longer
  grows with the bounding box

#### Scenario: The preview does not touch the exact stroke's pixels [pe_preview_is_presentational]

- **WHEN** a previewed stroke is recorded
- **THEN** the reduced-resolution work is discarded, the exact full-resolution
  stroke is computed before the history state is recorded, and every pixel of
  the committed document is byte-identical to the same stroke run without a
  preview

#### Scenario: Cancelling a previewed stroke restores the document [pe_preview_cancel]

- **WHEN** a previewed stroke is cancelled
- **THEN** the recorded samples are dropped without producing a history state
  and the displayed document is the pre-stroke one

### Requirement: The live stroke presents and rebuilds the pyramid frame-bounded

While a stroke is live the app SHALL present at most once per frame: the dab
that opens a frame presents immediately and the frame's later dabs accumulate
into one pending region, shown at the frame flush. The view pyramid SHALL be
rebuilt once per presented frame rather than once per dab, and each stored level
MUST equal a rebuild of the frame's level-0 buffer.

#### Scenario: A frame of dabs rebuilds the pyramid once [pe_pyramid_per_frame]

- **WHEN** several dabs arrive between two presents of a live stroke
- **THEN** the view pyramid is rebuilt once, at the flush, rather than once per
  dab

#### Scenario: The refreshed pyramid matches a rebuild [pe_pyramid_flush_equal]

- **WHEN** a frame's accumulated region is folded into the pyramid
- **THEN** every stored level equals a rebuild from the frame's level-0 buffer

### Requirement: Pencil paper grain

`StrokeConfig::grain` SHALL modulate the painted alpha of every dab by a fixed,
deterministic paper texture keyed to layer coordinates. The texture SHALL
combine a per-pixel hash with bilinear value noise on a 2 px and a 3 px grid so
the grain forms 2–3 px clumps with fine per-pixel variation, weighted toward
the clumps to match the 2B Pencil reference. It SHALL NOT punch holes in the
dab core, and a grainy stroke SHALL render identically on every run. Grainy
strokes SHALL stay on the CPU path because the GPU dab shader does not model
grain.

#### Scenario: Grain breaks up the dab alpha

- **WHEN** a dab is painted with grain 75 and the same dab with grain 0
- **THEN** the grained dab has partial alpha, no zero-alpha pixels in its core,
  at least half the plain coverage, less total coverage, and renders
  identically twice

#### Scenario: Grain strength follows the pencil presets

- **WHEN** a dab is painted with grain 50 and another with grain 55
- **THEN** the grain-50 dab keeps about 75 % of a plain dab's coverage and the
  grain-55 dab keeps about 72 %

#### Scenario: Grain is clumped

- **WHEN** grain noise is sampled across a layer
- **THEN** a 2 px box average keeps most of the texture's spread and
  horizontally adjacent pixels correlate at 0.55–0.72, so the grain forms
  2–3 px clumps rather than single-pixel static

### Requirement: Wet edges

`StrokeConfig::wet_edges` SHALL remap a Paint stroke's accumulated coverage
`a` through `f(a) = 1.3 · a · (1 − 0.76 · a)` clamped to 0..=1 before the
coverage is composited. A fully built-up stroke interior therefore paints
lighter than a single 50 %-flow dab, and the partially covered pixels at the
stroke's edge form a darker rim. Wet-edge strokes SHALL stay on the CPU path,
because the GPU dab shader does not model the curve. The curve is fitted to
the owner's Photoshop reference and is non-parity.

#### Scenario: Wet-edge transfer curve

- **WHEN** the wet-edge curve is evaluated at coverage 0, 128, and 255
- **THEN** it returns 0, about 40 % (103), and about 31 % (80), and it rises
  to a single peak before falling

#### Scenario: Wet edges lighten the core and darken the rim

- **WHEN** a hard 45 px oval stroke with Wet Edges paints `#47484a` at 35 %
  opacity and 50 % flow over white
- **THEN** the stroke interior lies 18–22 below white, a single dab's centre
  lies 24–28 below white, and the stroke's rim is darker than its interior

### Requirement: Clear zeroes RGB at full coverage

A `Clear` stroke at full opacity and full coverage SHALL write the pixel's RGB
to 0 as well as reducing its alpha to 0. At partial coverage the existing
colour SHALL remain, so a partly transparent pixel keeps a defined colour.

#### Scenario: Clear zeroes RGB at full coverage [pe_clear_zeroes_rgb]

- **WHEN** a Clear stroke paints at full opacity and full coverage on an opaque
  coloured pixel
- **THEN** the pixel's alpha becomes 0 and its RGB becomes 0

#### Scenario: Partial Clear keeps the colour [pe_clear_keeps_rgb]

- **WHEN** a Clear stroke paints at 50 % opacity on an opaque coloured pixel
- **THEN** the pixel's alpha falls but its RGB keeps the pre-stroke colour

### Requirement: Stroke dirtiness follows changed pixels

A stroke SHALL be dirty only for pixels whose stored bytes differ from their
pre-stroke values. Coverage that leaves every stored byte unchanged — Behind
over an opaque pixel, Opacity 0, Clear on a layer without alpha, or a clone
source outside the image — SHALL produce no dirty rectangle and therefore no
history state.

#### Scenario: A stroke that changes no pixel is a no-op [pe_noop_no_state]

- **WHEN** a Behind stroke paints over an opaque pixel, or a Normal stroke runs
  at Opacity 0
- **THEN** the stroke returns no dirty rectangle, adds no history state, and
  leaves the document not dirty
