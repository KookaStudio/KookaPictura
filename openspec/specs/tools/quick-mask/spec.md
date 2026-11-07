# tools/quick-mask Specification

## Purpose

Let users edit a selection as a temporary grayscale mask, preview its protection as a colored overlay, and convert the edited coverage back to an active selection without changing document pixels.

## Requirements

### Requirement: Entry initializes protection from the pixel selection

With a document open, the Quick Mask control and `Q` SHALL toggle Quick Mask mode. Entry SHALL preserve an existing pixel selection's exact coverage. Without an active pixel selection, entry SHALL show no protection anywhere under the default Masked Areas display. An explicitly empty selection SHALL remain distinct from no active selection. The default overlay SHALL be red at 50% opacity over fully protected areas, clear over fully selected areas, and proportional at partial coverage. Entry SHALL suppress marching ants for the selection used to initialize the mask. A temporary editing selection created inside Quick Mask SHALL display its own ordinary marching ants.

#### Scenario: No pixel selection starts clear [qm_entry_clear]
- **WHEN** Quick Mask is entered with a selected layer and no active pixel selection
- **THEN** the canvas has no red overlay, and black painting can create protected red areas anywhere on the canvas

#### Scenario: Existing selection protects its exterior [qm_entry_selection]
- **WHEN** Quick Mask is entered with a rectangular pixel selection
- **THEN** its interior is clear, its exterior has the red 50% overlay, and stored selection coverage is unchanged

#### Scenario: An explicit empty selection stays empty [qm_entry_empty]
- **WHEN** Quick Mask is entered with an explicit selection whose coverage is zero everywhere
- **THEN** the entire canvas is protected and tinted red at 50% opacity

### Requirement: Exit converts the edited mask to the active selection

Leaving Quick Mask SHALL remove its overlay and temporary channel and expose its edited coverage as the active pixel selection. Under Masked Areas, areas painted white SHALL be fully selected and clear, and areas painted black SHALL be unselected and red before exit. Every intermediate coverage byte SHALL survive without thresholding. Marching ants SHALL use the ordinary 50% coverage contour. An immediate entry/exit with no mask edits SHALL preserve the original selection, including the absence of an active selection.

#### Scenario: Black and white painting defines the new selection [qm_exit_selection]
- **WHEN** the user paints black and white regions in Quick Mask and presses `Q` again
- **THEN** white/clear regions are selected, black/red regions are excluded, and Quick Mask is off

#### Scenario: Soft coverage survives exit [qm_exit_soft]
- **WHEN** a mask containing coverage 0, 64, 127, 128, 192, and 255 is converted back to a selection
- **THEN** those exact bytes remain, while the outline follows the 50% contour

#### Scenario: Untouched toggles preserve selection identity [qm_exit_untouched]
- **WHEN** Quick Mask is entered and immediately left for no selection, an explicit empty selection, a full selection, or a soft selection
- **THEN** the original selection state is unchanged in each case

### Requirement: Mask painting is independent of layer pixels

Quick Mask SHALL route Brush, Pencil, Eraser, Paint Bucket, Gradient, and the existing grayscale-compatible mask filters to selection coverage. Under Masked Areas, black SHALL reduce coverage, white SHALL increase coverage, and brush opacity/flow and gray paint SHALL produce partial coverage. Supported paint blend modes SHALL operate on the grayscale destination in the gesture's captured polarity: coverage for Masked Areas and its inverse for Selected Areas, with results converted back to canonical coverage. Paint sources SHALL retain their grayscale values. Stroke and fill previews and commits SHALL use the same interpretation, including Normal and Multiply. Eraser SHALL paint the mask background value, with white revealing by default under Masked Areas. The pre-entry selection SHALL NOT clip its own mask edits. Unsupported pixel tools and color-only operations SHALL refuse instead of changing layer content. Move-tool content moves, Ctrl-drag content moves, Alt content duplication, and their preview entry points SHALL refuse in both the UI and backend without modifying the optional selection/coverage, layer masks, layer/composite pixels, layer target, or history, and SHALL leave no transient move preview. Entry from an existing content-move preview SHALL cancel that preview before presenting Quick Mask. Quick Mask routing SHALL take priority over an existing layer-mask edit target.

#### Scenario: Brush, Pencil and Eraser change coverage only [qm_paint_tools]
- **WHEN** Brush and Pencil paint black and Eraser paints the white background in Quick Mask
- **THEN** protection increases and then decreases at those positions, while layer pixels and the document composite remain byte-identical

#### Scenario: Partial paint creates partial selection coverage [qm_paint_partial]
- **WHEN** gray paint or a partially opaque brush is applied to a clear Quick Mask
- **THEN** intermediate coverage appears and survives exit to the selection

#### Scenario: Blend modes use the displayed mask convention [qm_blend_polarity]
- **WHEN** black Brush strokes and Paint Bucket fills in Normal and Multiply at full opacity/flow are applied independently to coverage 128 in each display polarity, with their available previews inspected before commit
- **THEN** Masked Areas produces exact coverage 0 and Selected Areas produces exact coverage 255 in both modes, preview and committed coverage agree, and layer/composite pixels and any layer mask remain unchanged

#### Scenario: Paint can expand beyond the original selection [qm_paint_unclipped]
- **WHEN** white is painted outside the selection used to enter Quick Mask
- **THEN** that formerly protected region becomes selected on exit

#### Scenario: Fill and filter operate on the mask [qm_fill_filter]
- **WHEN** Paint Bucket, Gradient, and a supported blur filter are applied in Quick Mask
- **THEN** only mask coverage changes, the preview shows the mask result, and document pixels remain unchanged

#### Scenario: Unsupported tools refuse safely [qm_unsupported_tool]
- **WHEN** a tool unsupported by the existing grayscale-mask path, such as Healing or Pattern Stamp, is invoked in Quick Mask
- **THEN** it refuses without changing coverage, pixels, or history

#### Scenario: Content moves and duplicates refuse without previews [qm_content_move_refused]
- **WHEN** Move-tool content moves, Ctrl-drag and Alt content duplication are attempted through UI gestures and direct backend preview/commit entry points in Quick Mask, with a pixel target and separately a raster layer-mask target
- **THEN** each attempt refuses, optional selection/coverage, layer masks, layer/composite pixels, layer target and history remain unchanged, and no moved or duplicate preview remains

#### Scenario: Entry cancels an existing content-move preview [qm_entry_move_preview]
- **WHEN** Quick Mask is entered while an uncommitted content-move or content-duplicate preview is active
- **THEN** the preview is cancelled to its pre-gesture selection and content baseline, its canvas offset and duplicate presentation are cleared, Quick Mask shows that baseline, and history is unchanged

#### Scenario: Quick Mask takes priority over a layer mask [qm_layer_mask_priority]
- **WHEN** Quick Mask is entered while a raster layer mask is the edit target and a supported painting operation is performed
- **THEN** selection coverage changes and the layer mask remains unchanged

### Requirement: Missing layer selection blocks painting

With Quick Mask active and no layer selected, a painting tool over the canvas SHALL display a blocked cursor. Painting input SHALL be rejected without changing coverage, document pixels, or history. Selecting a layer SHALL restore normal admission for supported Quick Mask painting tools.

#### Scenario: No active layer blocks the cursor and stroke [qm_no_layer]
- **WHEN** all layers are deselected in Quick Mask and a supported painting tool hovers and attempts a stroke on the canvas
- **THEN** the cursor is blocked and no mask edit, pixel edit, or history state occurs

#### Scenario: Selecting a layer enables painting again [qm_reselect_layer]
- **WHEN** a layer is selected after painting was blocked by an empty layer selection
- **THEN** the blocked cursor clears and a supported stroke changes Quick Mask coverage

### Requirement: Display controls never change coverage

Quick Mask SHALL default to Masked Areas with a red 50% overlay. Double-clicking its toolbar button SHALL open options for Masked Areas or Selected Areas, any overlay color, and opacity from 0 through 100%. Selected Areas SHALL tint selected coverage and reverse black/white paint interpretation without inverting existing coverage. `Alt`-click SHALL switch the same polarity setting, and the button glyph and dialog SHALL agree. `~` SHALL toggle the composite overlay and grayscale mask display. Display changes SHALL NOT alter coverage or create document history. A polarity change SHALL NOT reinterpret an in-progress stroke. Accepting options SHALL persist them in application preferences; cancelling SHALL retain the previous values. Entry SHALL initialize a separate Quick Mask foreground/background pair to black/white without overwriting the remembered image or layer-mask pairs. Exit SHALL restore the pair belonging to the surviving edit target: its remembered layer-mask pair when that target remains valid, otherwise its remembered image pair. Document activation SHALL project the newly active document's Quick Mask pair or current edit-target pair and SHALL preserve the remembered image and layer-mask pairs.

#### Scenario: Options affect appearance only [qm_options]
- **WHEN** color and opacity are changed, including 0% and 100%, in the Quick Mask Options dialog
- **THEN** the overlay follows those values and the mask coverage and history remain unchanged

#### Scenario: Dialog cancellation preserves settings [qm_options_cancel]
- **WHEN** the options dialog is edited and cancelled
- **THEN** the previous color, opacity, polarity, overlay, and persisted settings remain unchanged

#### Scenario: Polarity controls stay synchronized [qm_polarity]
- **WHEN** `Alt`-click switches to Selected Areas and the dialog is opened
- **THEN** the dialog and glyph show Selected Areas, the selected region is tinted, coverage is unchanged, and black paint selects while white paint protects

#### Scenario: Polarity cannot change during a stroke [qm_polarity_stroke]
- **WHEN** polarity is toggled while a stroke is in progress
- **THEN** the change takes effect only after the stroke has finished or been cancelled, and every dab in that stroke has the original interpretation

#### Scenario: Grayscale view and swatches follow the mode [qm_view_swatches]
- **WHEN** Quick Mask is entered, `~` is pressed twice, and Quick Mask is left
- **THEN** mask swatches start black/white, the view switches to grayscale and back, and the previous pair belonging to the surviving image or layer-mask edit target is restored on exit

#### Scenario: Quick Mask preserves remembered target colors [qm_target_swatches]
- **WHEN** distinct custom image and layer-mask foreground/background pairs are remembered, Quick Mask is entered from the layer-mask target, its own pair is changed while painting, another document is activated and the first is reactivated, then Quick Mask is exited and the first document's image target is selected
- **THEN** each activation projects the appropriate document and target's pair, reactivation of Quick Mask preserves its own pair, exit restores the custom layer-mask pair and original mask target, selecting the image target restores its custom image pair, and none of these pair transitions changes document pixels or history

#### Scenario: Options survive restarting the application [qm_preferences]
- **WHEN** accepted Quick Mask options are saved and the application is restarted
- **THEN** the next Quick Mask session uses those options and new documents still start in Standard mode

### Requirement: Mask edits use selection history

Each completed stroke, fill, or filter that changes Quick Mask coverage SHALL record one undoable state. Undo and redo SHALL restore exact coverage while Quick Mask is active and SHALL restore the corresponding selection after leaving the mode. Entering, leaving, and changing display settings SHALL add no extra states. Cancelling a stroke or filter preview SHALL restore its baseline and record nothing. Mode transitions SHALL settle a live stroke through the existing gesture lifecycle before changing its target.

#### Scenario: Undo works inside and outside the mode [qm_history]
- **WHEN** two mask edits are committed, undone and redone in Quick Mask, followed by exit and another undo/redo
- **THEN** each edit contributes one state, coverage and the resulting selection restore exactly, and toggles add no extra states

#### Scenario: Cancelling a mask edit restores its baseline [qm_cancel]
- **WHEN** a stroke or modal filter preview changes the displayed Quick Mask and is cancelled
- **THEN** coverage and the canvas return to their prior values and history is unchanged

### Requirement: Quick Mask has a document-local runtime lifecycle

Each open document SHALL retain its own Quick Mask mode and selection. Switching documents SHALL project the newly active document's mode, channel row, swatches, and highlight without leaking another document's overlay. Replacing or closing a document SHALL discard its Quick Mask presentation safely. The canvas SHALL present the mask at the document's coordinates during pan, zoom, and high-DPI drawing, without tinting exported images or other composite consumers. Quick Mask coverage SHALL use the existing 8-bit selection semantics regardless of the underlying image depth or color mode. Without GPU acceleration, the same mask and overlay semantics SHALL remain available.

#### Scenario: Document switching restores the correct mode [qm_documents]
- **WHEN** one document is edited in Quick Mask and another document is activated, then the first is reactivated
- **THEN** each document shows its own mode and mask without cross-document changes, and closing or replacing the first clears its temporary state

#### Scenario: Presentation follows the canvas transform [qm_presentation]
- **WHEN** a one-pixel or larger mask is viewed at different zooms, panned, and drawn at device pixel ratio 2 with GPU compute disabled
- **THEN** overlay and grayscale samples align with document pixels, coverage is unchanged, and non-canvas image consumers still receive the untinted composite

#### Scenario: Coverage is independent of image depth and mode [qm_depth_modes]
- **WHEN** supported RGB, Grayscale, CMYK, and Lab documents at their supported 8-, 16-, or 32-bit depths enter and leave Quick Mask with soft selection coverage
- **THEN** the exact 8-bit selection coverage is preserved and overlay tinting does not convert or mutate image pixels

### Requirement: Persistence is explicit through saved selections

Save and export while Quick Mask is active SHALL write the document image and its existing durable channels without a temporary Quick Mask channel, mode flag, or red overlay. Reopening SHALL start in Standard mode. After leaving Quick Mask, Save Selection SHALL persist the resulting coverage as an ordinary alpha channel through the existing channel command, and loading that channel SHALL recover the same selection.

#### Scenario: Active Quick Mask is absent from saved files [qm_save_temporary]
- **WHEN** a document is saved as PSD while Quick Mask contains unsaved coverage edits and then reopened
- **THEN** image pixels and durable channels are unchanged, there is no temporary Quick Mask channel, and Quick Mask is off

#### Scenario: Explicitly saved coverage round-trips [qm_save_alpha]
- **WHEN** an edited Quick Mask is exited, saved as an alpha channel, written to PSD, reopened, and loaded as a selection
- **THEN** the exact edited coverage is recovered

### Requirement: Quick Mask separates coverage from its editing selection

Quick Mask SHALL own mask coverage separately from the temporary pixel selection used to constrain editing. Entry SHALL use the original selection to initialize coverage and clear its cached marching ants. New selections SHALL show ordinary marching ants and constrain supported mask painting, fills and filters without replacing existing coverage. Deselect SHALL clear only that constraint. Exit with Q SHALL discard the temporary constraint and select the edited mask coverage. A document geometry operation applying its existing selection-reset policy SHALL clear both states so exit cannot install coverage with stale geometry.

#### Scenario: Cached entry outline disappears [qm_cached_ants]
- **WHEN** a selection outline has already been rendered and Quick Mask is entered
- **THEN** the old marching ants disappear immediately and its coverage remains in the mask

#### Scenario: Temporary selections constrain edits without replacing the mask [qm_temporary_clip]
- **WHEN** a new selection is made in Quick Mask and used for a fill, brush stroke or filter, then deselected or exited with Q
- **THEN** edits affect only the selected area, untouched mask coverage survives, deselection removes only the editing constraint, and Q exposes mask coverage rather than the constraint

#### Scenario: Geometry resets do not retain stale mask coverage [qm_geometry_reset]
- **WHEN** a document resize, canvas resize, rotation or crop clears the selection during Quick Mask and the mode is exited
- **THEN** no old-sized mask is installed as the selection, and undo restores the previous document and mask selection correctly

### Requirement: Quick Mask undo restores visible independent state

Undo and redo through keyboard and history commands SHALL restore mask coverage and temporary editing selections independently. The canvas SHALL repaint the restored mask even when document pixels have not changed. Mode toggles SHALL not add history states, and leaving Quick Mask SHALL not expose a temporary constraint during later undo or redo.

#### Scenario: Keyboard undo repaints the mask [qm_keyboard_undo]
- **WHEN** real canvas strokes are committed in Quick Mask and undone or redone through keyboard shortcuts
- **THEN** the history position, displayed overlay, and mask bytes agree without leaving the mode

#### Scenario: History preserves the mask and editing constraint separately [qm_independent_history]
- **WHEN** temporary selections and mask edits are undone and redone inside Quick Mask and across exit
- **THEN** both states restore correctly inside the mode, and only mask coverage becomes the normal selection outside it

### Requirement: Quick Mask strokes retain coverage with bounded presentation

Each brush stroke SHALL preserve existing mask coverage outside its edited samples. Live stroke samples SHALL use the canvas damage/deferred-present path without full shell refreshes, full-frame invalidation, document recompositing, or per-pixel Qt color queries. Coverage sharing SHALL avoid materializing a fragmented mask for a small dab. CPU presentation SHALL blend the same premultiplied overlay values as before.

#### Scenario: Existing mask survives consecutive canvas strokes [qm_retain_strokes]
- **WHEN** Quick Mask is initialized from a selection and several real brush gestures edit different locations
- **THEN** earlier mask regions and untouched coverage survive every stroke, and layer pixels remain unchanged

#### Scenario: Brush samples present bounded damage [qm_bounded_strokes]
- **WHEN** a small brush gesture produces multiple samples on a larger Quick Mask
- **THEN** samples emit bounded region damage, the gesture is recognized as painting, shell/frame invalidation waits for commit, and fragmented coverage is not materialized
