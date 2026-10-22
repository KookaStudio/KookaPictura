# tools/warp-text Specification

## Purpose
`Type > Warp Text`: the dialog, the live warp a type layer carries, how warped
type renders and round-trips through PSD, and the layers it refuses.

## ADDED Requirements

### Requirement: Warp Text opens from the Type menu and the type options bar

`Type > Warp Text…` SHALL be enabled exactly when the current layer is a type
layer, and SHALL open the Warp Text dialog for that layer. The four type tools'
options bars SHALL carry a Warp Text button that does the same and SHALL be
disabled when the current layer is not a type layer.

#### Scenario: The menu command opens the dialog for a type layer [wt_menu]

- **WHEN** `tst_warp_text::menuOpensDialog` makes a type layer current and
  dispatches the Warp Text command
- **THEN** the command is enabled and a `WarpTextDialog` opens for that layer

#### Scenario: The command is disabled without a type layer [wt_menu_disabled]

- **WHEN** `tst_warp_text::menuDisabledForPixelLayer` makes a pixel layer
  current
- **THEN** the Warp Text command and the options-bar Warp Text button are
  disabled

#### Scenario: The options-bar button opens the dialog [wt_options_button]

- **WHEN** `tst_warp_text::optionsButtonOpensDialog` selects the Horizontal
  Type tool with a type layer current and clicks the Warp Text button
- **THEN** a `WarpTextDialog` opens for that layer

### Requirement: The dialog offers CS6's warp controls

The dialog SHALL offer Style (None, then Arc, Arc Lower, Arc Upper, Arch,
Bulge, Shell Lower, Shell Upper, Flag, Wave, Fish, Rise, Fisheye, Inflate,
Squeeze, Twist), a Horizontal/Vertical choice, and Bend, Horizontal
Distortion, and Vertical Distortion, each an integer percentage from −100 to
+100. With Style None, the orientation and the three values SHALL be
disabled. Choosing a preset when the style was None and Bend is 0 SHALL set
Bend to +50. The dialog SHALL open showing the layer's current warp.

#### Scenario: The controls and their ranges [wt_controls]

- **WHEN** `tst_warp_text::dialogControls` opens the dialog on an unwarped
  type layer
- **THEN** Style lists None and the fifteen presets in order with None
  selected, the three values range −100…100, and orientation and values are
  disabled

#### Scenario: Picking a style starts at Bend +50 [wt_default_bend]

- **WHEN** `tst_warp_text::presetDefaultsBend` selects Arc from None
- **THEN** Bend reads +50 and the orientation and values are enabled

#### Scenario: Reopening shows the current warp [wt_reopen]

- **WHEN** `tst_warp_text::reopenShowsCurrentWarp` commits Wave, Vertical,
  Bend −30, H 10, V −20 and opens the dialog again
- **THEN** the dialog shows Wave, Vertical, −30, 10, and −20

### Requirement: The warp previews live and commits as one state

Every change in the dialog SHALL re-render the layer on the canvas without
recording history. OK SHALL record exactly one `Warp Text` history state, and
undoing it SHALL restore the pre-dialog layer. Cancel SHALL restore the
pre-dialog layer and record nothing. Committing Style None SHALL remove the
warp.

#### Scenario: Changes preview without history [wt_preview]

- **WHEN** `tst_warp_text::changesPreviewLive` opens the dialog and selects
  Arc
- **THEN** the layer's pixels change and the history state count is unchanged

#### Scenario: Cancel restores the layer [wt_cancel]

- **WHEN** `tst_warp_text::cancelRestoresLayer` previews Arc and cancels
- **THEN** the layer's pixels and warp equal the pre-dialog layer and no
  state was recorded

#### Scenario: OK records one undoable state [wt_commit_undo]

- **WHEN** `tst_warp_text::okRecordsOneState` commits Arc and then undoes
- **THEN** exactly one `Warp Text` state was added, and undo restores the
  unwarped pixels

#### Scenario: Style None removes the warp [wt_remove]

- **WHEN** `tst_warp_text::noneRemovesWarp` commits Arc and then commits None
- **THEN** the layer reports no warp and its pixels equal the never-warped
  layer's

### Requirement: Warped type renders from warped glyph outlines

A warped type layer SHALL render each glyph's outline mapped through the warp
over the text's untransformed box, then through the layer's transform, then
rasterized. The layer's rect SHALL bound the warped ink. A warp whose style
is None, or whose bend and distortions are all zero, SHALL render exactly as
unwarped type. Horizontal and Vertical SHALL warp along different axes, and a
distortion with Bend 0 SHALL still warp. Warped edges SHALL stay anti-aliased
rather than resampled.

#### Scenario: An identity warp is a no-op [wt_identity]

- **WHEN** `pictura_render::type_warp::tests::identity_warp_renders_unwarped`
  renders a spec with Style None, and with Arc at all-zero values
- **THEN** both buffers and rects equal the unwarped render

#### Scenario: Arc lifts the ends of a line [wt_arc]

- **WHEN** `pictura_render::type_warp::tests::arc_bends_the_line` renders
  one line with Arc, Bend +50
- **THEN** the ink in the middle column sits higher than the ink in the outer
  columns, and no ink falls outside the placement rect

#### Scenario: Orientation and distortion change the shape [wt_axes]

- **WHEN** `pictura_render::type_warp::tests::orientation_and_distortion_differ`
  renders Arc Horizontal, Arc Vertical, and Arc at Bend 0 with H Distortion
  50
- **THEN** all three differ from each other and from the unwarped render

#### Scenario: Warped edges stay sharp [wt_sharp]

- **WHEN** `pictura_render::type_warp::tests::warped_edges_stay_sharp` renders
  Flag at Bend 20
- **THEN** the share of partially covered ink pixels is within 1.5× the
  unwarped render's

#### Scenario: The warp applies before the layer transform [wt_transform]

- **WHEN** `pictura_render::type_warp::tests::warp_precedes_transform`
  renders a 90°-rotated Arc layer
- **THEN** its alpha equals the unrotated Arc render's alpha rotated by 90°,
  within one pixel at the edges

### Requirement: The warp stays live on the editable layer

Retyping a warped layer's text, changing its character or paragraph
attributes, applying a style, converting between point and paragraph type, and
transforming the layer SHALL keep its warp. While the Type tool edits a warped
layer, its preview SHALL be warped.

#### Scenario: Editing keeps the warp [wt_live_edit]

- **WHEN** `tst_warp_text::editKeepsWarp` warps a layer with Arc and retypes
  its text with the Type tool
- **THEN** the committed layer still reports Arc with Bend +50, and the edit
  preview was warped

#### Scenario: Attribute changes and transforms keep the warp [wt_live_attrs]

- **WHEN** `tst_warp_text::attributesAndTransformKeepWarp` warps a layer,
  changes its size in the Character panel, and scales it with Free Transform
- **THEN** the layer still reports the same warp after each change

### Requirement: The warp round-trips through PSD

Saving a type layer SHALL write its warp to the `TySh` warp descriptor as
`warpStyle`, `warpValue`, `warpPerspective`, `warpPerspectiveOther`, and
`warpRotate`. Opening a PSD SHALL decode that descriptor into the layer's live
warp. A style the engine cannot render (`warpCustom` or an unknown value)
SHALL decode as no warp for rendering while the descriptor bytes are kept.

#### Scenario: The descriptor is written [wt_psd_write]

- **WHEN** `pictura_codec::type_write::tests::authors_the_warp_descriptor`
  authors a spec warped with Fish, Vertical, Bend 40, H −10, V 25
- **THEN** the warp descriptor holds `warpFish`, 40, −10, 25 and `Vrtc`

#### Scenario: The descriptor is read [wt_psd_read]

- **WHEN** `pictura_codec::type_tool::tests::decodes_the_warp_descriptor`
  decodes descriptors for Arc Horizontal Bend 50, for `warpCustom`, and for
  an empty descriptor
- **THEN** they decode to Arc 50 Horizontal, None, and None

#### Scenario: A saved warp reopens live [wt_roundtrip]

- **WHEN** `pictura_render::type_warp::tests::warp_survives_save_and_open`
  adds a warped type layer, writes the PSD, and reads it back
- **THEN** `type_layer_spec` of the reopened layer reports the same warp

### Requirement: Faux Bold, outline-less fonts, and type on a path are refused

Warp Text SHALL refuse a layer whose type is on a path (on-path or area), whose
character attributes use Faux Bold, or whose font has no outline data (no
`glyf`, `CFF `, or `CFF2` table). It SHALL show an alert naming the reason,
not open the dialog, and leave the layer and the history unchanged.

#### Scenario: The refusal check [wt_refusal_rules]

- **WHEN** `pictura_render::type_warp::tests::refusals` checks a plain
  spec, a Faux Bold spec, an on-path spec, an area spec, and a spec whose face
  has its outline tables stripped
- **THEN** only the plain spec is warpable, and each other spec reports its
  reason

#### Scenario: Faux Bold shows an alert [wt_faux_bold]

- **WHEN** `tst_warp_text::fauxBoldIsRefused` invokes Warp Text on a Faux Bold
  type layer
- **THEN** a warning naming Faux Bold is shown, no dialog opens, and no state
  is recorded

#### Scenario: Type on a path shows an alert [wt_on_path]

- **WHEN** `tst_warp_text::typeOnPathIsRefused` invokes Warp Text on an on-path
  type layer
- **THEN** a warning naming the path is shown, no dialog opens, and no state
  is recorded

### Requirement: A warped type layer is marked in the Layers panel

The Layers panel SHALL report a type layer with a non-identity warp as
warped, and SHALL draw its thumbnail as the type card with a bent baseline
under the "T".

#### Scenario: The row reports the warp [wt_thumbnail]

- **WHEN** `tst_warp_text::layersRowMarksWarp` commits Arc on a type layer and
  then commits None
- **THEN** the layer's row reports warped after the first commit and unwarped
  after the second
