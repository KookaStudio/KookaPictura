# tools/transform-selection Specification

## Purpose
Lets the user reshape the active selection border — move, scale, rotate, skew,
distort, or warp its coverage — without moving the pixels the selection
encloses.

## Requirements

### Requirement: Transform Selection entry and enablement

`Select > Transform Selection` SHALL be an implemented, live command that begins
a selection-border transform session. It SHALL be enabled when a document is
open, a non-empty selection exists, and no modal transform or other blocking
document session is active. It SHALL be disabled with no document, with no
selection, with an empty selection, and while a modal transform is pending.
Starting the session SHALL record no history state.

#### Scenario: Enabled with a selection [ts_menu_enabled]

- **WHEN** a document is open with a non-empty selection and no modal session
- **THEN** `Select > Transform Selection` is enabled

#### Scenario: Disabled without a selection [ts_menu_no_selection]

- **WHEN** a document is open with no selection, or with an all-zero selection
- **THEN** `Select > Transform Selection` is disabled and invoking it changes nothing

#### Scenario: Disabled while a modal transform is pending [ts_menu_modal]

- **WHEN** a Free Transform session is already pending
- **THEN** `Select > Transform Selection` is disabled and does not replace the pending session

### Requirement: Selection-border transform session

Beginning a selection-border transform SHALL place the shared transform box on
the selection's coverage bounds and SHALL offer the same interactions as Free
Transform: the Free, Scale, Rotate, Skew, Distort, and Perspective gestures;
the movable reference point; numeric X/Y position, W/H percentages with the
aspect link, angle, and H/V skew; the interpolation choice; and the exact
rotate/flip commands while the session is open. The session SHALL NOT lift,
move, clear, or otherwise change any layer pixel or the selection mask before
commit. Beginning again on the same selection while its session is open SHALL
be a no-op; a begin while another kind of document session owns the slot SHALL
be refused.

#### Scenario: Begin puts the box on the selection bounds [ts_begin]

- **WHEN** `Select > Transform Selection` is invoked on a non-empty selection
- **THEN** the transform box spans the selection's coverage bounds, the transform options page is shown, and the history count is unchanged

#### Scenario: Shift corner drag scales the border proportionally [ts_scale_shift]

- **WHEN** a corner handle is dragged with Shift during a border session
- **THEN** the border scales about the reference point with its aspect ratio preserved, and the committed border's bounds match the drag within 1 px

#### Scenario: Numeric width doubles the border [ts_width_200]

- **WHEN** W is entered as 200 % with the aspect link on
- **THEN** the committed border's width and height are twice the original selection bounds within 1 px

#### Scenario: A corner reference point stays fixed under rotation [ts_rotate_pivot]

- **WHEN** the reference point is set to the top-left locator and the border is rotated 90°
- **THEN** the committed border's top-left source corner stays within 1 px of the reference point

### Requirement: Border commit resamples the coverage mask

Committing a selection-border transform SHALL resample the pre-transform
coverage mask once through the session's live map, clipped to the canvas, and
SHALL replace the document selection with the result. The commit SHALL record
exactly one history state labelled `"Transform Selection"` for a non-identity
transform. Committing an identity transform SHALL record nothing and leave the
mask byte-identical. The layer channels SHALL remain byte-identical before and
after every border commit. Undo SHALL restore the pre-transform coverage byte
for byte. A feathered or anti-aliased selection SHALL keep partial coverage
values where the resample produces them. A result that lies entirely outside
the canvas SHALL commit an empty selection and remain undoable.

#### Scenario: Commit records one state and replaces the border [ts_commit_state]

- **WHEN** a border session with a non-identity transform is committed
- **THEN** the selection equals the resampled border and exactly one `"Transform Selection"` state is recorded

#### Scenario: The underlying pixels do not change [ts_pixels_untouched]

- **WHEN** a border transform is committed over a pixel layer with content
- **THEN** every channel byte of every layer equals its pre-command value

#### Scenario: Undo restores the border exactly [ts_undo]

- **WHEN** a border transform is committed and then undone
- **THEN** the coverage mask equals the pre-transform mask byte for byte

#### Scenario: An identity commit records nothing [ts_identity]

- **WHEN** the session is committed without changing the box
- **THEN** the selection is unchanged and no history state is added

#### Scenario: A border moved fully off the canvas commits empty [ts_off_canvas]

- **WHEN** the box is dragged until no coverage remains inside the canvas and committed
- **THEN** the selection has no covered pixels, one state is recorded, and undo restores the previous coverage

#### Scenario: Soft coverage stays soft [ts_soft]

- **WHEN** a feathered selection is scaled during the session and committed
- **THEN** boundary pixels still hold coverage strictly between 0 and 255 where the resample produces a ramp

### Requirement: Cancel restores the border

Escape, the options-bar Cancel button, and the document-change "Don't Apply"
choice SHALL end a selection-border session without changing the selection and
without adding a history state.

#### Scenario: Escape restores the pre-command border [ts_escape]

- **WHEN** a border transform is in progress and Escape is pressed
- **THEN** the session ends, the selection is byte-identical to its pre-command value, and no history state is added

### Requirement: Live border preview

While a selection-border session is live, the canvas SHALL draw marching ants
on the border produced by the live transform and SHALL NOT draw the
pre-transform committed border. Each gesture update SHALL record no history.
The committed border SHALL be re-traced from the resampled mask at the 50 %
coverage line.

#### Scenario: The ants follow the drag [ts_preview_follows]

- **WHEN** a handle is dragged during a border session
- **THEN** the canvas shows the transformed border and no pre-transform border, and the history count stays unchanged

#### Scenario: The committed outline follows the mask [ts_contour_50]

- **WHEN** a border transform is committed
- **THEN** the marched outline traces the resampled mask's 50 % coverage contour within 0.5 px

#### Scenario: The border preview is not a history state [ts_preview_only]

- **WHEN** a border session is cancelled after several gestures
- **THEN** the selection and the history count equal their pre-session values

### Requirement: Warp on the selection border

A selection-border session SHALL support Warp with the same mesh, presets,
orientation, bend/distortion values, and custom anchor/surface drags as Free
Transform, applied over the selection bounds. The border preview SHALL follow
the warp. Leaving Warp SHALL keep the border where it is and move the box to
the warped bounds. Applying SHALL warp the coverage through the mesh and then
the session's box frame, including a Distort or Perspective made before Warp,
as one transform. Cancelling SHALL leave the mask unchanged.

#### Scenario: Warp enters on the selection bounds [ts_warp_enter]

- **WHEN** Warp is entered during a border session
- **THEN** the warp mesh spans the selection bounds, the warp fields are shown, and the border is unchanged

#### Scenario: A preset and a custom drag reshape the preview [ts_warp_edit]

- **WHEN** a warp preset is chosen, or an anchor or the mesh surface is dragged during a border session
- **THEN** the border preview follows the warped coverage and no history state is added

#### Scenario: Apply warps the mask once [ts_warp_commit]

- **WHEN** a warped border session is applied
- **THEN** the selection becomes the warped coverage, exactly one `"Transform Selection"` state is recorded, and the layer pixels are unchanged

#### Scenario: Leaving Warp keeps the border in place [ts_warp_leave]

- **WHEN** Warp mode is left before the session is applied
- **THEN** the border preview does not move and the transform box hugs the warped bounds

#### Scenario: Warp composes with a prior Distort or Perspective [ts_warp_compose]

- **WHEN** a Distort or Perspective gesture is applied and Warp is then entered and applied
- **THEN** the committed border is the coverage warped by the mesh and then mapped by the box frame

### Requirement: Border transform is distinct from Free Transform

`Edit > Free Transform` and `Ctrl+T` with an active selection SHALL keep
transforming the selected pixels and SHALL drop the selection on commit.
`Select > Transform Selection` SHALL change only the border and SHALL keep the
selection active. The two commands SHALL NOT share a target: starting one SHALL
not silently apply the other.

#### Scenario: Ctrl+T still transforms pixels [ts_distinct_free]

- **WHEN** `Ctrl+T` is invoked with an active selection
- **THEN** the selected pixels are lifted and transformed, and the selection is dropped on commit

#### Scenario: Transform Selection keeps the pixels and the selection [ts_distinct_border]

- **WHEN** `Select > Transform Selection` is invoked with the same selection
- **THEN** only the border changes, the image is untouched, and the selection remains active

### Requirement: Modal lock during a border transform

While a selection-border transform is pending, the Select menu commands SHALL
be disabled so the mask cannot change under the session, in addition to the
existing Layer, Image, Filter, and pixel-edit lock. A tool switch, layer change,
or document change SHALL use the same Apply / Don't Apply / Cancel prompt as a
layer transform: Apply commits the border, Don't Apply cancels it, and Cancel
keeps the session pending.

#### Scenario: Select commands are disabled while pending [ts_lock_select]

- **WHEN** a selection-border transform is pending and the Select menu is opened
- **THEN** Deselect, Inverse, and the Modify commands are disabled

#### Scenario: A tool switch prompts before dropping the border [ts_prompt]

- **WHEN** the user switches tools during a pending border transform
- **THEN** Apply commits the border, Don't Apply cancels it, and Cancel leaves the session and the selection unchanged
