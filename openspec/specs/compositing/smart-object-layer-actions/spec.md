# smart-object-layer-actions Specification

## Purpose
Convert to Smart Object, author embedded sources, preserve on save and load, and rasterize, each as one undo state.

## Requirements

### Requirement: Embedded smart-object source authoring

On success the operation SHALL author an embedded source document and attach it
to the layer as `SmartObject { kind: Embedded, payload, filename, filetype,
creator }`. The source document SHALL use the document's color mode and bit
depth and the layer's `rect` as its size, SHALL contain a copy of the layer
translated so its top-left is `(0, 0)`, and SHALL carry a merged composite equal
to the layer's raster over that rect. The operation SHALL serialize the source
with `write_psd` and store the bytes as the payload, with `filename` equal to
`"<layer name>.psd"`, `filetype` `8BPB`, and `creator` `8BIM`. The payload SHALL
be non-empty and SHALL be a valid PSD that the codec resolves as an embedded
smart object.

#### Scenario: The payload is a valid embedded source

- **WHEN** a raster layer is converted and the stored payload is read with `read_psd`
- **THEN** the payload parses, its document size equals the layer's `rect`, and its merged composite equals the layer's raster

#### Scenario: SmartObject metadata is stamped

- **WHEN** the conversion succeeds
- **THEN** the layer's smart object reports kind `Embedded`, a non-empty payload, filename `"<layer name>.psd"`, `filetype` `8BPB`, and `creator` `8BIM`

#### Scenario: Same mode and depth

- **WHEN** a Grayscale 8-bit document's layer is converted
- **THEN** the embedded source document is Grayscale 8-bit and `write_psd` accepts it

### Requirement: The raster proxy and rendering are unchanged

The operation SHALL keep the layer's existing pixel channels as a raster proxy
and, except when the target is the Background, SHALL NOT change the layer's name,
rect, blend, opacity, fill, mask, or position. When the target is the Background,
the operation SHALL apply the unlock described by `A Background is unlocked
before conversion` — renaming the layer and adding an opaque alpha channel — and
SHALL otherwise leave the layer's rect, blend, opacity, fill, mask, and position
unchanged. Because a layer with a proxy renders from the proxy, the document
composite after conversion SHALL equal the composite before conversion.

#### Scenario: Pixel channels survive

- **WHEN** a raster layer that is not the Background and has color channels is converted
- **THEN** the layer's channels are byte-for-byte the same as before

#### Scenario: The composite is unchanged

- **WHEN** the service composite is compared before and after a successful conversion
- **THEN** the two composites are equal

#### Scenario: Rejected destinations are untouched beyond the refusal

- **WHEN** conversion is refused
- **THEN** the layer's channels, rect, name, and smart-object state are unchanged

### Requirement: Save and load preserve the converted object

A document containing a converted layer SHALL round-trip through `write_psd`
and `read_psd` with the smart object preserved as embedded and its payload
unchanged.

#### Scenario: Round-trip preserves the object

- **WHEN** a document with a converted layer is written and read back
- **THEN** the layer resolves as an embedded smart object with a non-empty payload whose bytes equal the written payload

#### Scenario: The proxy still round-trips

- **WHEN** a converted document is written and read back
- **THEN** the layer's pixel channels still resolve and the composite is unchanged

### Requirement: The app command records exactly one undo state

The application SHALL expose the command `Layer > Smart Objects > Convert to
Smart Object` with the stable id `LayerSmartObjectConvertTo`. The command SHALL
be enabled only when the current layer is convertible, SHALL invoke the engine
operation on success, clear the link sets, recomposite, and record exactly one
history state labelled `"Convert to Smart Object"`. A refusal SHALL record no
history state.

#### Scenario: Success records one state

- **WHEN** the command runs on a convertible current layer
- **THEN** exactly one history state labelled `"Convert to Smart Object"` is added and the layer reports a smart object

#### Scenario: A Background success records one state

- **WHEN** the command runs on the Background layer
- **THEN** exactly one history state labelled `"Convert to Smart Object"` is added, the layer is no longer the Background, and it reports a smart object

#### Scenario: Refusal records nothing

- **WHEN** the command runs on a group, an adjustment layer, or an existing smart object
- **THEN** the history count is unchanged and the document is unchanged

#### Scenario: Enabled only when convertible

- **WHEN** the current layer is not convertible
- **THEN** the command is disabled and its availability predicate mutates nothing

### Requirement: Rasterize Smart Object eligibility and refusal

The engine operation `pictura-render::rasterize_smart_object(doc, path)` SHALL
resolve `path` and SHALL rasterize only when the target is a single
smart-object layer: the path resolves, the layer is not a group, has no
adjustment data, and `smart_object.is_some()`. On any other target the operation
SHALL return `false` and SHALL NOT mutate the document. When materialization
requires decoding the embedded payload (the layer has no color channel) and the
payload is empty or cannot be decoded, the operation SHALL also return `false`
with no mutation.

#### Scenario: A smart-object layer rasterizes

- **WHEN** `rasterize_smart_object` is called on the path of a non-group, non-adjustment layer carrying a smart object
- **THEN** it returns `true` and the layer no longer carries a smart object

#### Scenario: A group is refused

- **WHEN** the target path resolves to a group layer
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An adjustment layer is refused

- **WHEN** the target layer has adjustment data
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: A non-smart layer is refused

- **WHEN** the target layer has no smart object
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An unresolved path is refused

- **WHEN** `path` does not resolve to a layer
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An undecodable payload is refused

- **WHEN** the target is a channel-less smart-object layer whose embedded payload is empty or cannot be decoded
- **THEN** the operation returns `false` and the document is unchanged

### Requirement: Rasterizing materializes the object content into layer channels

On success the operation SHALL materialize the object's rendered content into
the layer's pixel channels and SHALL set `layer.smart_object` to `None`. When
the layer already has a color channel (`id == 0`) and no eligible active stack
mode applies, that existing raster proxy IS the content, so the operation SHALL
leave every pixel channel byte-for-byte unchanged and SHALL NOT decode the
payload. When a layer with a color channel carries an eligible active stack mode
(an embedded payload with at least two visible frames and every enabled Smart
Filter decodes), the operation SHALL materialize the reduced stack source into
channels `0..mode.color_channels()` plus a `-1` alpha channel. When a layer with
a color channel carries a stack mode whose stack source does not apply, the
operation SHALL keep the proxy byte-for-byte. When the layer has no color
channel, the operation SHALL decode the embedded payload through the existing
embedded-source render path, scale the decoded source into the layer's `rect`,
and write channels `0..mode.color_channels()` plus a `-1` alpha channel; when
that layer carries a stack mode, the source is the reduced stack render when the
reduced stack source applies, otherwise the existing embedded source. The
operation SHALL NOT change the layer's name, `rect`, blend, opacity, fill, mask,
or position.

#### Scenario: A proxy is left unchanged
- **WHEN** a smart-object layer with a color channel and no eligible active stack mode is rasterized
- **THEN** the layer's pixel channels are byte-for-byte the same as before

#### Scenario: A source-only layer materializes from the payload
- **WHEN** a smart-object layer with no color channel but a decodable embedded payload is rasterized
- **THEN** the layer's color channels and `-1` alpha hold the decoded source scaled into the layer `rect`

#### Scenario: A stack-mode proxy bakes the reduction
- **WHEN** a proxied smart-object layer with an eligible active stack mode is rasterized
- **THEN** its pixel channels hold the reduced stack source and the layer no longer carries a smart object

#### Scenario: A non-applicable stack mode keeps the proxy
- **WHEN** a proxied smart-object layer carries a stack mode for which the reduced stack source does not apply
- **THEN** its pixel channels are byte-for-byte the same as before

#### Scenario: The smart object is cleared
- **WHEN** rasterizing succeeds
- **THEN** `layer.smart_object` is `None`

#### Scenario: Layout fields are untouched
- **WHEN** a layer is rasterized
- **THEN** its name, `rect`, blend, opacity, fill, mask, and position are unchanged

### Requirement: Rasterize drops the preserved smart-object blocks and linked record

On success the operation SHALL remove the layer's preserved config block
(`SoLd`/`SoLE`/`plLd`/`PlLd`) from `layer.extra_blocks` and SHALL remove the
matching document-level linked-source record from `Document.layer_section_extra`,
so that a subsequent save does not re-emit a smart object and does not carry an
orphan `lnk*` record. A re-save followed by a load SHALL resolve no smart object
on the rasterized layer.

#### Scenario: The config block is gone

- **WHEN** a layer that carried an `SoLd` config block is rasterized
- **THEN** no `SoLd`/`SoLE`/`plLd`/`PlLd` block remains in `layer.extra_blocks`

#### Scenario: The linked record is gone

- **WHEN** a rasterized document is saved
- **THEN** its preserved section carries no linked-source record for the rasterized layer

#### Scenario: Save then load resolves no smart object

- **WHEN** a rasterized document is written and read back
- **THEN** the layer has no smart object and the document carries no orphan `lnk*` record

### Requirement: Codec removes a linked-source record by uuid

The codec function `remove_linked_source(layer_section_extra, uuid) -> Option<Vec<u8>>` SHALL
walk the preserved top-level tagged blocks. For a `lnkD`/`lnk2`/`lnk3`/
`lnkE` block it SHALL parse the `u64`-length-prefixed record list, remove the
record(s) whose Pascal-string uuid equals `uuid`, rebuild that block with the
same key, a recomputed length, and even padding, and copy every other block
byte-for-byte. It SHALL return `Some(new_bytes)` when at least one record was
removed and `None` when nothing matched, leaving the caller's bytes untouched.
It SHALL never panic; a malformed block SHALL be copied verbatim.

#### Scenario: A matching record is removed

- **WHEN** a section holds a `lnk2` with two records and an unrelated block, and one record's uuid is removed
- **THEN** the call returns `Some`, the surviving record and the unrelated block are byte-preserved, and the removed uuid no longer appears

#### Scenario: A non-matching uuid returns none

- **WHEN** no record's uuid equals the requested uuid
- **THEN** the call returns `None`

#### Scenario: A malformed block does not panic

- **WHEN** a tagged block's record list is malformed
- **THEN** the call returns without panicking and leaves that block verbatim

### Requirement: The Rasterize Smart Object command records exactly one undo state

The application SHALL expose the command `Layer > Rasterize > Smart Object` with
the stable id `LayerRasterizeSmartObject`. The command SHALL be enabled only when
the current layer is a rasterizable smart-object layer, SHALL invoke the engine
operation on success, then clear the link sets, recomposite, and record exactly
one history state labelled `"Rasterize Smart Object"`. A refusal SHALL record no
history state.

#### Scenario: Success records one state

- **WHEN** the command runs on a rasterizable smart-object current layer
- **THEN** exactly one history state labelled `"Rasterize Smart Object"` is added and the layer no longer reports a smart object

#### Scenario: Refusal records nothing

- **WHEN** the command runs on a non-smart layer, a group, or an adjustment layer
- **THEN** the history count is unchanged and the document is unchanged

#### Scenario: Enabled only for a rasterizable smart-object layer

- **WHEN** the current layer is not a rasterizable smart-object layer
- **THEN** the command is disabled and its availability predicate mutates nothing

### Requirement: Place Smart Object from a file

The engine operation `pictura-render::place_smart_object(doc, filename, bytes)` SHALL decode `bytes`
with `pictura_codec::read_psd` and SHALL return `None`
without mutating the document when `bytes` do not parse as a PSD/PSB document.
Its signature is `place_smart_object(doc: &mut Document, filename: &str, bytes:
&[u8]) -> Option<String>`. On success it SHALL append a new layer at the top of
the document, name it `filename`, and return the new layer's path using the same
path string convention as the other `layer_ops` (`"0"`, `"2/1"`, …). The
operation SHALL NOT change any existing layer or the document size. The
operation SHALL append the layer at the source's native rect; the canvas fit
required of the Place command is a separate non-destructive layer transform
applied by the application after the smart object exists.

#### Scenario: A PSD source is placed

- **WHEN** `place_smart_object` is called with the bytes of a valid PSD and a display name
- **THEN** it returns `Some(path)` for a new topmost layer and mutates no existing layer

#### Scenario: Malformed bytes are refused

- **WHEN** `place_smart_object` is called with bytes that are not a parseable PSD/PSB document
- **THEN** it returns `None` and the document is unchanged

### Requirement: A placed smart-object layer is channel-less, source-native, and renders from its embedded source

For a decoded source of size `w × h`, the appended engine layer MUST have `rect
= (0, 0, w, h)`, `visible` true, `BlendMode::Normal`, `opacity` 255, no pixel
channels, and `smart_object = Some(SmartObject { kind: Embedded, payload:
Some(bytes), filename, filetype: *b"8BPB", creator: *b"8BIM", .. })`. This
native rect is the engine primitive; the Place command's canvas fit is the
separate non-destructive transform below. Because the layer has no color
channel, the compositor SHALL render it from its embedded payload through the
existing embedded-source path, and the composite SHALL show the source content
within the layer rect. A `write_psd` followed by `read_psd` SHALL re-resolve the
placed layer as an embedded smart object with an unchanged payload.

#### Scenario: The layer geometry and metadata are stamped

- **WHEN** `place_smart_object` appends a source of size `w × h` as `filename`
- **THEN** the new topmost layer is named `filename`, has `rect = (0, 0, w, h)`, is visible, Normal, opacity 255, has no pixel channels, and carries an `Embedded` smart object with `payload` equal to the source bytes, `filename`, `filetype` `8BPB`, and `creator` `8BIM`

#### Scenario: The composite shows the placed source

- **WHEN** a solid-colour PSD is placed into a document
- **THEN** `composite_rgba` shows the source colour within the placed layer's rect

#### Scenario: Save then load preserves the placed object

- **WHEN** a document with a placed layer is written with `write_psd` and read back
- **THEN** the layer resolves as an embedded smart object whose payload equals the written bytes

### Requirement: Oversized placed sources are fitted to the canvas non-destructively

The Place command SHALL create each placed layer as a smart object first: a
PSD/PSB source retains its original bytes and native pixel dimensions, and a
raster source is converted at its native pixel dimensions, before applying any
canvas fit. When the source's native width exceeds the document canvas width or
its native height exceeds the document canvas height, the command SHALL fit the
layer by a uniform scale of
`s = min(document_width / source_width, document_height / source_height, 1.0)`
and SHALL centre the fitted layer on the canvas. The fitted layer's rect
dimensions SHALL be `round(source_width × s)` and `round(source_height × s)`,
each at least 1 px. The fit SHALL change the layer's transform/rect only: it
MUST NOT resample, crop, replace, or discard the embedded source or its native
dimensions. A source that already fits both axes SHALL be placed at 1:1; the
command MUST NOT upscale it. The fit SHALL be part of the same `"Place"` history
state and MUST NOT add a separate history state.

For the raster route, the source SHALL be converted to the smart object at its
native dimensions before the fit; the embedded source MUST NOT be authored from
a downscaled copy. The Free Transform session started by Place SHALL begin from
the fitted rect. The fit and a scale or translation committed in that session
SHALL be non-destructive for the placed smart object: the `smart_object` and its
payload SHALL remain present, and a later scale-up SHALL sample the unchanged
original source pixels. A rotation, skew, or warp SHALL use the existing
rasterizing transform path and is outside this non-destructive rule. The
composite SHALL clip at the document canvas when the transformed layer extends
beyond it. Escape or Cancel SHALL still remove the placement as defined by
`free-transform`.

#### Scenario: An oversized landscape source is fitted [place_fit_landscape]

- **WHEN** a 4000 × 3000 PSD source is placed into a 1000 × 1000 document
- **THEN** the placed layer is an embedded smart object whose rect is 1000 × 750 centred on the canvas, its payload is the original source bytes, and its source dimensions remain 4000 × 3000

#### Scenario: An oversized portrait source is fitted [place_fit_portrait]

- **WHEN** a 3000 × 4000 source is placed into a 1000 × 800 document
- **THEN** the placed layer's rect is 600 × 800, the scale is uniform, and it is centred on the canvas

#### Scenario: A source that already fits is not upscaled [place_fit_native]

- **WHEN** a 640 × 480 source is placed into a 1000 × 1000 document
- **THEN** the placed layer's rect is 640 × 480 and the embedded source keeps its 640 × 480 dimensions

#### Scenario: A fitted raster keeps its native source [place_fit_raster_native]

- **WHEN** a 4000 × 3000 raster image is placed into a 1000 × 1000 document
- **THEN** the embedded source is authored from the full 4000 × 3000 raster, the layer rect is 1000 × 750, and no downscaled source is stored

#### Scenario: Scaling a fitted placement up keeps the original source [place_fit_enlarge]

- **WHEN** a 4000 × 3000 source is fitted into a 1000 × 1000 document and the Place Free Transform session scales it to 2× and commits
- **THEN** the layer rect is 2000 × 1500, the smart object and its original payload and 4000 × 3000 source dimensions remain, and the composite is clipped to the 1000 × 1000 canvas

#### Scenario: The fit is part of the Place state [place_fit_state]

- **WHEN** an oversized source is placed and its Free Transform session is committed unchanged
- **THEN** exactly one `"Place"` state records the fitted layer and no separate fit state exists

### Requirement: The Place Smart Object command records exactly one undo state

The application SHALL expose the command `File > Place…` with the stable id
`file.place`. The command SHALL open a file dialog offering an `Images (…)`
filter beside the existing Photoshop filter entries (`*.psd *.psb`), SHALL accept
a multi-file selection, and SHALL be enabled only when a document is open and no
transform session is active. A dispatch of the command without an active
document or while a transform session is active MUST refuse before opening the
dialog. The dialog SHALL preserve the rest of the existing filters,
last-directory behaviour, and per-file routing.

For each chosen path, in the order the dialog returns, the command SHALL route a
`*.psd`/`*.psb` file through the bridge `place_smart_object(path) -> QString`
and every other supported raster image through the bridge `place_image(path) ->
QString` defined by the `image-import` capability; a PSD/PSB file MUST NOT be
routed through Qt. For a Photoshop file the bridge SHALL read the file, derive
the display name from its base name, and invoke the engine operation on the
current document. On success only, the invoked bridge SHALL clear the link sets,
apply the canvas fit required for an oversized source, recomposite, record
exactly one history state labelled `"Place"`, and return the new layer path. The
fit SHALL be part of that same state and MUST NOT add a separate history state.
On refusal the bridge SHALL return an empty string, record no history state, and
leave the document unchanged; the command SHALL continue with the remaining
selected paths.

After each successful place the application SHALL select the new layer and start
a Free Transform session on it from the fitted rect as defined by the
`free-transform` capability.
For a menu Place selection, the next selected file SHALL be placed only after
the current session is committed or ended. Cancelling the session with Escape or
the transform's Cancel control SHALL cancel that placement as defined by
`free-transform`; a session that ends by a tool switch or another
non-interruption cancel SHALL keep the placed layer and its `"Place"` state and
SHALL continue with the next selected file. If an undo already removed the
`"Place"` state, the command SHALL treat the item as cancelled without a second
undo. A profile-warning cancellation SHALL skip only that file. Each raster
file's profile decision SHALL apply only to that file; a failed placement SHALL
NOT carry its pending profile policy to the next file.

After the selection finishes, if one or more files could not be placed (missing,
unreadable, undecodable, unsupported, over budget, or refused by the placement),
the application SHALL show one summary dialog naming those files. A
profile-warning cancellation, a transform cancellation, and a cancelled batch
SHALL NOT be listed as placement failures. A cancelled batch SHALL stop placing
further files and SHALL NOT show the summary.

#### Scenario: Success records one state and returns the layer path

- **WHEN** the command runs with a valid PSD file and an open document
- **THEN** exactly one history state labelled `"Place"` is added, the layer count increases by one, and the bridge returns the new layer's path

#### Scenario: A raster image records one state and returns the layer path

- **WHEN** the command runs with a supported raster image and an open document
- **THEN** exactly one history state labelled `"Place"` is added, a new smart-object layer is appended, and the image-import bridge returns the new layer's path

#### Scenario: A successful place starts a transform session

- **WHEN** the command places a supported image and returns the new layer path
- **THEN** a Free Transform session begins on the new layer from its fitted rect and the `"Place"` history state remains recorded

#### Scenario: Refusal records nothing

- **WHEN** the command runs with a malformed source file
- **THEN** the history count is unchanged, the document is unchanged, and the bridge returns an empty string

#### Scenario: Enabled only when a document is open

- **WHEN** no document is open
- **THEN** the command is disabled

#### Scenario: The Place dialog offers the Images filter and multi-select [place_dialog_filters]

- **WHEN** `File > Place…` opens with a document active
- **THEN** the dialog offers an `Images (…)` filter beside the Photoshop entries and accepts a multi-file selection

#### Scenario: Multiple files place one layer per file [place_multi]

- **WHEN** the user selects two supported files in `File > Place…` and commits each Free Transform session
- **THEN** each file adds one layer and one `"Place"` state, the files are placed in dialog order, and no transform session remains at the end

#### Scenario: Mixed formats stay on their own routes [place_multi_mixed]

- **WHEN** the selection contains a PSD/PSB file and a raster image
- **THEN** the PSD/PSB file is placed through `place_smart_object` with its original bytes embedded, and the raster is placed through `place_image`

#### Scenario: A file that cannot be placed does not stop the batch [place_multi_partial]

- **WHEN** the selection contains one undecodable file and supported files
- **THEN** the supported files are placed, the undecodable file records no layer or history state, and one summary dialog names it after the batch

#### Scenario: All files failing shows one summary [place_multi_all_failed]

- **WHEN** every selected file cannot be placed
- **THEN** no layer or history state is added and one summary dialog names every failed file

#### Scenario: A profile-warning cancel skips only that file [place_multi_cancelled]

- **WHEN** the user cancels a raster file's profile warning in a multi-file selection
- **THEN** that file records nothing, the remaining files are placed, and the summary dialog does not list the cancelled file

#### Scenario: A failed file's profile choice does not leak [place_multi_profile_policy]

- **WHEN** a raster file's profile warning is accepted and that file then fails to place, and the next raster file is placed
- **THEN** the next file uses its own profile decision or the default, not the failed file's choice

#### Scenario: Place is unavailable without an active document or during a transform [place_multi_no_document]

- **WHEN** no document is active or a Free Transform session is active
- **THEN** the `File > Place…` command is disabled, dispatch does not open the Place dialog or start a placement, and `placeDocuments` refuses at entry

### Requirement: Replace Smart Object Contents eligibility and refusal

The engine operation SHALL resolve `path` and SHALL replace the source only
when the target is a single replaceable smart-object layer: the path resolves,
the layer is not a group, has no adjustment data, and its `smart_object` is
`Embedded` with a payload. Its signature is
`pictura-render::replace_smart_object_contents(doc, path, filename, bytes) ->
bool`. The operation SHALL decode `bytes` with `pictura_codec::read_psd` and
SHALL refuse when they do not parse as a PSD/PSB document. On any refusal it
SHALL return `false` and SHALL NOT mutate the document.

#### Scenario: A replaceable smart-object layer replaces its contents

- **WHEN** `replace_smart_object_contents` is called on a non-group, non-adjustment layer whose smart object is embedded with a payload and `bytes` are a valid PSD/PSB document
- **THEN** it returns `true`

#### Scenario: A non-smart layer is refused

- **WHEN** the target layer has no smart object
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: A group is refused

- **WHEN** the target path resolves to a group layer
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An adjustment layer is refused

- **WHEN** the target layer has adjustment data
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An embedded object without a payload is refused

- **WHEN** the target layer's smart object is `Embedded` but has no payload
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: Malformed bytes are refused

- **WHEN** the target is a replaceable smart object and `bytes` do not parse as a PSD/PSB document
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An unresolved path is refused

- **WHEN** `path` does not resolve to a layer
- **THEN** the operation returns `false` and the document is unchanged

### Requirement: Replacing contents swaps only the embedded source and clears the proxy

On success the operation SHALL set the layer's `smart_object.payload` to the new
source `bytes`, set `filename` to the supplied name, set `filetype` to `8BPB`
and `creator` to `8BIM`, and clear the stored `uuid` to empty. It SHALL clear
the layer's pixel channels so the compositor renders the new source through the
existing embedded-source path scaled into the layer's existing `rect`. It SHALL
preserve the layer's transform/geometry and every other property: `rect`, name,
mask, blend, opacity, fill, color, and lock flags SHALL be unchanged. Only the
smart-object source and the pixel channels SHALL change.

#### Scenario: The source metadata is replaced

- **WHEN** a replaceable smart object is replaced with `bytes` and `filename`
- **THEN** its payload equals `bytes`, its filename equals `filename`, and its filetype/creator are `8BPB`/`8BIM`

#### Scenario: The stored uuid is cleared

- **WHEN** the replacement succeeds
- **THEN** the layer's smart object `uuid` is empty

#### Scenario: The proxy is cleared so the new source renders

- **WHEN** a proxy-backed smart object is replaced and the document is composited
- **THEN** the layer has no pixel channels and the compositor draws the new source scaled into the layer's rect

#### Scenario: Geometry and layout are untouched

- **WHEN** a smart object with a mask, a non-default blend, opacity, fill, color, and lock flags is replaced
- **THEN** its `rect`, name, mask, blend, opacity, fill, color, and lock flags are unchanged

### Requirement: Replace drops and re-authors the preserved blocks and linked record

On success the operation SHALL remove the layer's preserved config block
(`SoLd`/`SoLE`/`plLd`/`PlLd`) from `layer.extra_blocks` and, when the previous
`uuid` was non-empty, SHALL remove the matching document-level linked-source
record from `Document.layer_section_extra` via
`pictura_codec::remove_linked_source`. A subsequent `write_psd` followed by
`read_psd` SHALL resolve the layer as an embedded smart object whose payload is
the NEW source bytes, and the old uuid and old payload SHALL be absent from the
saved document.

#### Scenario: The config block is gone

- **WHEN** a layer that carried an `SoLd` config block is replaced
- **THEN** no `SoLd`/`SoLE`/`plLd`/`PlLd` block remains in `layer.extra_blocks`

#### Scenario: The old linked record is gone

- **WHEN** a replaced document is inspected
- **THEN** the old uuid no longer appears in `layer_section_extra`

#### Scenario: Save then load resolves the new source

- **WHEN** a replaced document is written and read back
- **THEN** the layer resolves as an embedded smart object whose payload equals the new source, not the old payload or uuid

### Requirement: The Replace Contents command records exactly one undo state

The application SHALL expose the command `Layer > Smart Objects > Replace
Contents…` with the stable id `LayerSmartObjectReplaceContents` and SHALL show a
file dialog filtered to Photoshop files (`*.psd *.psb`). The command SHALL be
enabled only when the current layer is a replaceable smart object. On a chosen
path the bridge `replace_smart_object_contents(path, file_path) -> bool` SHALL
read the file, derive the display name from its base name, and invoke the engine
operation on the current document. On success only, the bridge SHALL clear the
link sets, recomposite, and record exactly one history state labelled
`"Replace Contents"`. On refusal the bridge SHALL return `false`, record no
history state, and leave the document unchanged.

#### Scenario: Success records one state

- **WHEN** the command runs on a replaceable smart-object current layer with a valid PSD/PSB file
- **THEN** exactly one history state labelled `"Replace Contents"` is added and the layer's payload is the new source

#### Scenario: Refusal records nothing

- **WHEN** the command runs with a missing, unreadable, or malformed file, or on a non-replaceable layer
- **THEN** the history count is unchanged, the document is unchanged, and the bridge returns `false`

#### Scenario: Enabled only for a replaceable smart object

- **WHEN** the current layer is not a replaceable smart object
- **THEN** the command is disabled and its availability predicate mutates nothing

### Requirement: Open As Smart Object decodes a PSD/PSB source or refuses

The engine operation SHALL decode `bytes` with `pictura_codec::read_psd`; its
signature is `pictura-render::open_as_smart_object(filename: &str, bytes:
&[u8]) -> Option<Document>`. It SHALL return `None` when `bytes` do not parse as
a PSD/PSB document. On success it SHALL return `Some(doc)` where `doc` is a new
document holding the source as a smart object. The operation SHALL NOT mutate
any input.

#### Scenario: A PSD source opens

- **WHEN** `open_as_smart_object` is called with the bytes of a valid PSD/PSB document and a display name
- **THEN** it returns `Some(doc)`

#### Scenario: Malformed bytes are refused

- **WHEN** `open_as_smart_object` is called with bytes that are not a parseable PSD/PSB document
- **THEN** it returns `None`

### Requirement: An opened-as-smart-object document is new, source-sized, and holds one embedded layer

The returned document SHALL be built with `Document::new(src.width,
src.height, src.mode, src.depth)`, preserving the source's dimensions, color
mode, and bit depth. Its merged composite SHALL be set from the decoded source's
composite so the pre-render composite shows the source. It SHALL contain exactly
one layer, appended by the existing
`place_smart_object(&mut doc, filename, bytes)`: a topmost, channel-less,
native-size layer at `(0, 0)` carrying `SmartObject { kind: Embedded, payload:
Some(bytes), filename, filetype: 8BPB, creator: 8BIM, .. }`. Because the layer
has no color channel, the compositor SHALL render it from its embedded payload.

#### Scenario: Geometry, mode, and depth match the source

- **WHEN** a source of size `w × h` in a given color mode and bit depth is opened
- **THEN** the returned document has `width = w`, `height = h`, and the source's mode and depth

#### Scenario: Exactly one embedded smart-object layer

- **WHEN** a valid PSD/PSB source is opened
- **THEN** the returned document has exactly one layer, its `smart_object.kind` is `Embedded`, its payload equals the source `bytes`, and its name is the supplied display name

#### Scenario: The composite shows the source colour

- **WHEN** a solid-colour PSD is opened as a smart object and the document is composited
- **THEN** `composite_rgba` shows the source colour

### Requirement: The Open As Smart Object bridge opens an untitled document and records one undo state

The bridge `PictureView::open_as_smart_object(path: &QString) -> bool` SHALL
read `path`, derive the source display name from the file's base name, and call
the engine operation with those bytes. On success it SHALL store the composited
image and the document, reset the edit state, capture exactly one history state
labelled `"Open As Smart Object"`, and set the view's path to NONE so the new
document is untitled. It SHALL return `false` without mutating the view when the
file is missing, unreadable, or not a PSD/PSB document.

#### Scenario: Success stores one untitled document

- **WHEN** the bridge is called with a valid PSD/PSB file
- **THEN** it returns `true`, exactly one history state labelled `"Open As Smart Object"` is added, and the view holds one document with one smart-object layer

#### Scenario: A refused source mutates nothing

- **WHEN** the bridge is called with a missing, unreadable, or malformed file
- **THEN** it returns `false`, the current document is unchanged, and no history state is added

#### Scenario: The opened document is untitled so Save cannot overwrite the source

- **WHEN** a source file is opened as a smart object
- **THEN** the view's path is empty and the source file is left untouched

### Requirement: The Open As Smart Object command creates a new untitled tab

The application SHALL expose `File > Open As Smart Object…` with the stable id
`FileOpenAsSmartObject` (`"file.openAsSmartObject"`). The command SHALL open a
file dialog filtered to Photoshop files (`*.psd *.psb`) and SHALL be enabled
whenever the application is running, like File Open. On a chosen path it SHALL
mirror `PicturaMainWindow::openPath`: construct a new `PictureView`, call
`open_as_smart_object`, and on success call `addDocument(view, QString())` so a
new untitled tab is created. On refusal it SHALL discard the view and add no
tab.

#### Scenario: The dialog filters to PSD/PSB and needs no open document

- **WHEN** the command is available with no document open
- **THEN** it is enabled and opens a file dialog filtered to `*.psd *.psb`

#### Scenario: Success adds one untitled tab

- **WHEN** the command is dispatched with a valid PSD/PSB path
- **THEN** exactly one new tab is added and its document path is empty

#### Scenario: Refusal adds no tab

- **WHEN** the command is dispatched with a malformed source
- **THEN** no tab is added and the document count is unchanged

### Requirement: Export Smart Object Contents accessor reads the embedded payload

The engine function `pictura-render::smart_object_source_bytes` SHALL accept
`(doc: &Document, path: &str) -> Option<Vec<u8>>`, SHALL resolve `path`, and SHALL
return `Some(payload.clone())` only when the resolved layer is not a group, has no
adjustment data, and its `smart_object` is `Some` with a non-empty payload. On
any other target it SHALL return `None`. The function SHALL be a pure read and
SHALL NOT mutate the document.

#### Scenario: A smart-object layer yields its payload

- **WHEN** `smart_object_source_bytes` is called on the path of a non-group, non-adjustment layer carrying a non-empty embedded payload
- **THEN** it returns `Some(bytes)` where `bytes` equal that layer's payload

#### Scenario: A non-smart layer yields none

- **WHEN** the target layer has no smart object
- **THEN** the function returns `None`

#### Scenario: A group yields none

- **WHEN** the target path resolves to a group layer
- **THEN** the function returns `None`

#### Scenario: An adjustment layer yields none

- **WHEN** the target layer has adjustment data
- **THEN** the function returns `None`

#### Scenario: An empty payload yields none

- **WHEN** the target layer's smart object has no payload or an empty payload
- **THEN** the function returns `None`

#### Scenario: An unresolved path yields none

- **WHEN** `path` does not resolve to a layer
- **THEN** the function returns `None`

#### Scenario: The document is unchanged

- **WHEN** `smart_object_source_bytes` returns a payload and the document is compared with a pre-call clone
- **THEN** the two documents are equal

### Requirement: The Export Contents bridge writes the payload byte-for-byte

The bridge `PictureView::export_smart_object_contents` SHALL accept
`(path: &QString, dest: &QString) -> bool`, SHALL read the source bytes for
`path` through `smart_object_source_bytes`, and SHALL write them to `dest` with
`std::fs::write`, without transcoding, so the destination holds the payload
exactly as stored. It SHALL return `true` only when the write succeeds. A
`None` source or a write failure SHALL return `false` and SHALL produce no
side effects beyond the attempted write.

#### Scenario: A successful export writes the exact bytes

- **WHEN** the current layer is a smart object with a payload and `dest` is a writable path
- **THEN** the bridge returns `true` and the file at `dest` is byte-for-byte the layer's payload

#### Scenario: A missing payload writes nothing and returns false

- **WHEN** `path` resolves to a layer without a non-empty payload
- **THEN** the bridge returns `false` and no file is written

#### Scenario: A failed write returns false

- **WHEN** the payload is available but `dest` cannot be written
- **THEN** the bridge returns `false`

### Requirement: The Export Contents command records no history state

The application SHALL expose the command `Layer > Smart Objects > Export
Contents…` with the stable id `LayerSmartObjectExportContents`
(`"layer.smartObject.exportContents"`). The command SHALL be enabled only when
the current layer is a smart object with a non-empty payload. Its handler SHALL
show a save dialog with a `*.psd` default filter, call
`export_smart_object_contents` for the current layer and the chosen destination,
and report the result. Because export only reads the document, the command SHALL
record no history state, SHALL NOT clear the link sets, and SHALL NOT
recomposite.

#### Scenario: Enabled only for a smart object with a payload

- **WHEN** the current layer is a smart object with a non-empty payload
- **THEN** the command is enabled, and it is disabled when the current layer is a non-smart layer, a group, or an adjustment layer

#### Scenario: The command is registered with its stable id

- **WHEN** the command registry is built
- **THEN** `LayerSmartObjectExportContents` is registered as an implemented `Layer > Smart Objects > Export Contents…` command

#### Scenario: Exporting adds no history state and mutates nothing

- **WHEN** the command runs on a smart-object current layer and a destination is chosen
- **THEN** the destination file holds the payload, the history count is unchanged, and the document is unchanged

#### Scenario: A cancelled or failed export records nothing

- **WHEN** the save dialog is cancelled or the bridge returns `false`
- **THEN** no history state is added and the document is unchanged

### Requirement: Edit Contents eligibility and refusal

The application SHALL enable Edit Contents for a non-group, non-adjustment
layer with an embedded PSD/PSB source that parses or a supported embedded PDF
or PDF-compatible AI source. A read-only eligibility check SHALL reject every
other target and SHALL NOT mutate the document. A refused dispatch SHALL
record no history state.

#### Scenario: A PSD/PSB source is editable

- **WHEN** `layer_can_edit_smart_object_contents` is called on a non-group, non-adjustment layer whose `Embedded` smart object has a payload that parses as a PSD/PSB document
- **THEN** it returns `true`

#### Scenario: A non-parsing source is refused

- **WHEN** the target layer carries a non-empty payload of an unsupported format such as a placed JPEG
- **THEN** the predicate returns `false` and the command is disabled

#### Scenario: A non-embedded object is refused

- **WHEN** the target layer's smart object is not of kind `Embedded` (for example an external, alias, or unresolved record)
- **THEN** the predicate returns `false` and the command is disabled

#### Scenario: A non-smart layer is refused

- **WHEN** the target layer has no smart object
- **THEN** the predicate returns `false`

#### Scenario: A group or adjustment layer is refused

- **WHEN** the target path resolves to a group or a layer with adjustment data
- **THEN** the predicate returns `false`

#### Scenario: An empty payload is refused

- **WHEN** the target layer's smart object has no payload or an empty payload
- **THEN** the predicate returns `false`

#### Scenario: An unresolved path is refused

- **WHEN** `path` does not resolve to a layer
- **THEN** the predicate returns `false`

#### Scenario: The predicate mutates nothing

- **WHEN** the predicate is called on any target and the document is compared with a pre-call clone
- **THEN** the two documents are equal

### Requirement: The Edit Contents command opens the source as a new untitled editor document

The application SHALL expose `Layer > Smart Objects > Edit Contents` with the
stable id `LayerSmartObjectEditContents` (`"layer.smartObject.editContents"`).
The command SHALL be enabled according to Edit Contents eligibility. When
dispatched on an editable PSD/PSB source it SHALL export the target's embedded payload byte-for-byte to a
per-session temporary file, load that file into a new document as an ordinary
document (its own layers and composite), and add it through
`addDocument(editor, QString())` so a new untitled tab opens. The editor document
SHALL be the decoded source, NOT a document wrapping the source as a smart-object
layer, and the editor tab SHALL be untitled regardless of the temporary file's
path.

#### Scenario: The command is registered with its stable id

- **WHEN** the command registry is built
- **THEN** `LayerSmartObjectEditContents` is registered as an implemented `Layer > Smart Objects > Edit Contents` command

#### Scenario: Enabled only for an editable source

- **WHEN** the current layer is an editable smart object
- **THEN** the command is enabled, and it is disabled when the current layer is a non-smart layer, a group, an adjustment layer, a non-`Embedded` object, an object with an empty payload, or an unsupported payload

#### Scenario: One new untitled editor tab opens

- **WHEN** the command runs on an editable PSD/PSB smart-object layer
- **THEN** exactly one tab is added, its document path is empty, and it holds the decoded source content

#### Scenario: Opening records no state on the origin

- **WHEN** the command opens an internal PSD/PSB editor
- **THEN** the origin document is unchanged and its history count is unchanged

#### Scenario: An ineligible target adds no tab

- **WHEN** the command is dispatched while the current layer is not editable
- **THEN** no tab is added, no history state is recorded, and no origin state changes

### Requirement: Saving an editor commits the source and records exactly one Edit Contents undo state

The bridge SHALL expose `commit_smart_object_edit(layer_path, file_path) ->
bool`. It SHALL apply the same engine operation as
`replace_smart_object_contents` (read `file_path`, decode it as a PSD/PSB
source, swap the origin layer's embedded source, clear its pixel proxy, and drop
the preserved config block and matching linked-source record) and, on success
only, SHALL clear the link sets, recomposite, and record exactly one history
state labelled `"Edit Contents"`. The existing
`replace_smart_object_contents` method SHALL keep its signature, behavior, and
`"Replace Contents"` label. Saving an editor tab SHALL be intercepted in the
application's Save path: it SHALL call `editor->save(temp_file)` then
`origin->commit_smart_object_edit(layer_path, temp_file)`, refresh, and return
success WITHOUT opening a Save dialog and WITHOUT writing any user-chosen file.
The editor tab SHALL remain open and SHALL be marked clean. On any refusal the
bridge SHALL return `false`, record no history state, and leave the origin
unchanged.

#### Scenario: Save commits and records one state

- **WHEN** an editor tab is saved after the document was edited
- **THEN** the origin layer's embedded payload is the edited source and exactly one history state labelled `"Edit Contents"` is added

#### Scenario: The origin recomposites to the edited source

- **WHEN** the committed origin is composited
- **THEN** the original layer paints the edited source scaled into its existing rect

#### Scenario: The editor stays open and clean

- **WHEN** saving an editor commits successfully
- **THEN** the editor tab stays open and reports no unsaved changes

#### Scenario: Save writes no user file and shows no dialog

- **WHEN** Save runs on a session editor
- **THEN** no Save dialog is shown, no user-chosen file is written, and the only file written is the session temporary file

#### Scenario: Refusal records nothing

- **WHEN** the edited document cannot be serialized or the commit is refused
- **THEN** the bridge returns `false`, the origin is unchanged, and its history count is unchanged

#### Scenario: Replace Contents still records its own label

- **WHEN** `replace_smart_object_contents` runs on a replaceable layer
- **THEN** it records exactly one `"Replace Contents"` state, unchanged from before

### Requirement: Discarding or closing an Edit Contents session leaves the origin unchanged

Closing an editor tab without saving SHALL leave the origin document unchanged
and SHALL record no history state. Closing an editor SHALL remove its session and
delete its temporary file. Closing the origin SHALL remove every session
targeting it and SHALL leave each orphaned editor open as an ordinary untitled
document; a later Save on an orphaned editor SHALL behave like Save As for an
untitled document and SHALL NOT write the deleted temporary file. Destroying the
window SHALL delete every session's temporary file.

#### Scenario: Closing the editor unsaved leaves the origin unchanged

- **WHEN** an editor tab is closed without saving
- **THEN** the origin document is unchanged and its history count is unchanged

#### Scenario: Closing the editor removes the session and its temp file

- **WHEN** an editor tab is closed
- **THEN** the session temporary file no longer exists and the session is dropped

#### Scenario: Closing the origin orphans the editor as untitled

- **WHEN** the origin document is closed while an editor for it is open
- **THEN** the editor tab stays open with an empty document path and no session

#### Scenario: An orphaned editor does not write the deleted temp file

- **WHEN** Save runs on an orphaned editor
- **THEN** it routes through Save As and no file is written to the deleted temporary path

#### Scenario: Window destruction cleans up temp files

- **WHEN** the main window is destroyed with live sessions
- **THEN** every session's temporary file is deleted

### Requirement: Reset Smart Object Transform eligibility and refusal

The engine operation `pictura-render::reset_smart_object_transform(doc, path)`
SHALL resolve `path` and SHALL act only on a single smart-object layer: the path
resolves, the layer is not a group, has no adjustment data, and its
`smart_object` is `Embedded` with a non-empty payload that parses as a PSD/PSB
document (so its native size is known). On any other target, or a payload that
does not parse, the operation SHALL return `false` and SHALL NOT mutate the
document.

#### Scenario: A transformed embedded object can be reset

- **WHEN** `can_reset_smart_object_transform` is called on the path of a non-group, non-adjustment layer whose `Embedded` smart object has a payload that parses as PSD/PSB
- **THEN** it returns `true`

#### Scenario: A group, an adjustment, or a non-smart layer is refused

- **WHEN** the target path resolves to a group, a layer with adjustment data, or a layer with no smart object
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: A non-embedded or unparseable source is refused

- **WHEN** the target layer's smart object is not `Embedded`, or its non-empty payload does not parse as a PSD/PSB document
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An unresolved path is refused

- **WHEN** `path` does not resolve to a layer
- **THEN** the operation returns `false` and the document is unchanged

### Requirement: Resetting restores the source's native transform

On success the operation SHALL read the embedded source document's native size
`w × h` and SHALL set the layer's `rect` to `(0, 0, w, h)`, i.e. the source's
native transform at the origin. It SHALL clear the layer's pixel channels so the
compositor renders the embedded source again at native scale and rotation, and
SHALL keep `layer.smart_object` so the object stays editable. It SHALL NOT
change the layer's name, blend, opacity, fill, mask, or lock flags. The
operation SHALL be non-destructive: no transform history is stored and the
source payload is unchanged.

#### Scenario: The rect returns to the native size at the origin

- **WHEN** a smart object whose `rect` was scaled or moved is reset
- **THEN** its `rect` is `(0, 0, source_width, source_height)`

#### Scenario: The composite shows the unrotated native source

- **WHEN** a reset object is composited
- **THEN** the layer paints the embedded source with no scale or rotation, clipped to the document

#### Scenario: The object stays a smart object

- **WHEN** resetting succeeds
- **THEN** `layer.smart_object` is still `Some` and its payload is byte-for-byte unchanged

#### Scenario: Layout fields are untouched

- **WHEN** a smart object with a mask, non-default blend, opacity, fill, and lock flags is reset
- **THEN** its name, blend, opacity, fill, mask, and lock flags are unchanged

### Requirement: The Reset Transform command records exactly one undo state

The application SHALL expose `Layer > Smart Objects > Reset Transform` with the
stable id `LayerSmartObjectResetTransform` (`"layer.smartObject.resetTransform"`).
The command SHALL be enabled only when the current layer can be reset, SHALL
invoke the engine operation on success, then clear the link sets, recomposite,
and record exactly one history state labelled `"Reset Transform"`. A refusal
SHALL record no history state.

#### Scenario: Success records one state

- **WHEN** the command runs on a resettable smart-object current layer
- **THEN** exactly one history state labelled `"Reset Transform"` is added and the layer's `rect` is its native size at the origin

#### Scenario: Refusal records nothing

- **WHEN** the command runs on a non-smart layer, a group, an adjustment layer, or an object whose payload does not parse
- **THEN** the history count is unchanged and the document is unchanged

#### Scenario: Enabled only for a resettable object

- **WHEN** the current layer is not a resettable smart object
- **THEN** the command is disabled and its availability predicate mutates nothing

### Requirement: Convert Smart Object to Layers eligibility and refusal

The engine operation `pictura-render::convert_smart_object_to_layers(doc, path)`
SHALL resolve `path` and SHALL act only on a single smart-object layer: the path
resolves, the layer is not a group, has no adjustment data, and its
`smart_object` is `Embedded` with a non-empty payload that parses as a PSD/PSB
document. On any other target, or a payload that does not parse, or a source
document with a zero dimension, the operation SHALL return `false` and SHALL NOT
mutate the document.

#### Scenario: An embedded object converts

- **WHEN** `convert_smart_object_to_layers` is called on a non-group, non-adjustment layer whose `Embedded` smart object has a payload that parses as a PSD/PSB document
- **THEN** it returns `true`

#### Scenario: A group, an adjustment, or a non-smart layer is refused

- **WHEN** the target path resolves to a group, a layer with adjustment data, or a layer with no smart object
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: A non-embedded or unparseable source is refused

- **WHEN** the target layer's smart object is not `Embedded`, or its non-empty payload does not parse as a PSD/PSB document
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An unresolved path is refused

- **WHEN** `path` does not resolve to a layer
- **THEN** the operation returns `false` and the document is unchanged

### Requirement: Converting to layers splices the source stack into place

On success the operation SHALL decode the embedded source and SHALL replace the
smart-object layer with the source document's layers. Each source layer SHALL be
mapped from the source's `(0, 0, w, h)` frame into the object's `rect`: a source
`rect` `r` becomes the document rect `obj.left + r.left * (obj.width / w)`,
`obj.top + r.top * (obj.height / h)`, and so on, and a pixel layer's channel
planes SHALL be resampled to the mapped rect (a same-size, origin-`(0, 0)` map
SHALL copy bytes unchanged). A source document with no layers SHALL become a
single raster layer covering the object's `rect`, built from the source's merged
composite. The replacement layers SHALL occupy the object's slot in the stack;
the object layer SHALL be removed. The matching document-level linked-source
record SHALL be removed so a re-save does not carry an orphan record.

#### Scenario: An untransformed object yields its source stack at the object position

- **WHEN** a smart object whose `rect` is `(left, top, left + w, top + h)` for a source of size `w × h` is converted
- **THEN** the object layer is replaced by the source's layers, each at its source `rect` translated by `(left, top)`

#### Scenario: A scaled object scales its source stack

- **WHEN** a smart object whose `rect` is half or double the source size is converted
- **THEN** the replacement layers' rects are scaled by the same factor and their channel planes are resampled into the mapped rects

#### Scenario: A payload with no layers becomes one raster layer

- **WHEN** the embedded source document carries no layers but has a merged composite
- **THEN** the object layer is replaced by one raster layer covering the object's `rect` whose channels hold the source composite

#### Scenario: The linked record is dropped

- **WHEN** a converted-from object that carried a linked record is written
- **THEN** the saved document carries no linked-source record for the consumed object

### Requirement: The Convert to Layers command records exactly one undo state

The application SHALL expose `Layer > Smart Objects > Convert to Layers` with the
stable id `LayerSmartObjectConvertToLayers`
(`"layer.smartObject.convertToLayers"`). The command SHALL be enabled only when
the current layer can be converted to layers, SHALL invoke the engine operation
on success, then clear the link sets, recomposite, and record exactly one history
state labelled `"Convert to Layers"`. A refusal SHALL record no history state.

#### Scenario: Success records one state and replaces the object

- **WHEN** the command runs on a convertible smart-object current layer
- **THEN** exactly one history state labelled `"Convert to Layers"` is added and the layer count changes by the source layer count minus one

#### Scenario: Refusal records nothing

- **WHEN** the command runs on a non-smart layer, a group, an adjustment layer, or an object whose payload does not parse
- **THEN** the history count is unchanged and the document is unchanged

#### Scenario: Enabled only for a convertible object

- **WHEN** the current layer is not a convertible smart object
- **THEN** the command is disabled and its availability predicate mutates nothing

### Requirement: New Smart Object via Copy gives an independent embedded source

The engine operation `pictura-render::new_smart_object_via_copy(doc, path)` SHALL
resolve `path` and SHALL act only on a single non-group, non-adjustment layer
whose `smart_object` is `Embedded` with a non-empty payload. On any other target
it SHALL return `None` and SHALL NOT mutate the document. On success it SHALL
deep-clone the layer, name the copy `"<name> copy"`, clear the copy's preserved
config descriptor and `uuid` so the copy does not share a preserved `SoLd`/`SoLE`
link record with the original, keep the (deep-copied) payload, insert the copy
directly above the original in its container, and return the copy's path using
the same path-string convention as the other `layer_ops`. The original SHALL be
unchanged, and editing either object's source SHALL NOT change the other.

#### Scenario: The copy is inserted above and owns its payload

- **WHEN** `new_smart_object_via_copy` roots at a copyable smart object
- **THEN** a new layer directly above it carries an `Embedded` smart object whose payload equals the original's, whose `uuid` is empty, and whose name is `"<name> copy"`

#### Scenario: The original is unchanged

- **WHEN** the copy is created
- **THEN** the original layer's payload, smart object, and preserved blocks are byte-for-byte unchanged

#### Scenario: Editing one source leaves the other alone

- **WHEN** the copy's embedded source is replaced and both layers' smart objects are compared
- **THEN** the original's payload is unchanged and the two payloads differ

#### Scenario: A non-smart or non-embedded target is refused

- **WHEN** the target has no smart object, is a group, has adjustment data, or is not an `Embedded` object with a payload
- **THEN** the operation returns `None` and the document is unchanged

### Requirement: The New Smart Object via Copy command records exactly one undo state

The application SHALL expose `Layer > Smart Objects > New Smart Object via Copy`
with the stable id `LayerSmartObjectNewViaCopy`
(`"layer.smartObject.newViaCopy"`). The command SHALL be enabled only when the
current layer is a copyable smart object, SHALL invoke the engine operation on
success, then clear the link sets, recomposite, and record exactly one history
state labelled `"New Smart Object via Copy"`, and SHALL return the copy's path so
the caller can select it. A refusal SHALL return an empty path and record no
history state.

#### Scenario: Success records one state and returns the copy path

- **WHEN** the command runs on a copyable smart-object current layer
- **THEN** exactly one history state labelled `"New Smart Object via Copy"` is added, the layer count increases by one, and the bridge returns the new layer's path

#### Scenario: Refusal records nothing

- **WHEN** the command runs on a non-smart layer, a group, an adjustment layer, or a non-embedded object
- **THEN** the history count is unchanged, the document is unchanged, and the bridge returns an empty string

#### Scenario: Enabled only for a copyable object

- **WHEN** the current layer is not a copyable smart object
- **THEN** the command is disabled and its availability predicate mutates nothing

### Requirement: Smart-filter list document operations

The engine SHALL expose `pictura_render::add_smart_filter(doc, path, filter_id,
name, options) -> bool`, `delete_smart_filter(doc, path, index) -> bool`,
`reorder_smart_filters(doc, path, from, to) -> bool`, and
`clear_smart_filters(doc, path) -> bool`. Each SHALL resolve `path` and SHALL act
only on a layer that is not a group, has no adjustment data, and carries a smart
object; otherwise it SHALL return `false` without mutating the document. On an
eligible layer the operations SHALL delegate to the codec descriptor edits so the
preserved `filterFX` list and the typed `smart_filters` view change together.
`add_smart_filter` SHALL attach the filter enabled, replacing an existing entry
with the same `filter_id`. A refusal from the codec SHALL return `false` without
mutating.

#### Scenario: Adding appends or replaces a filter

- **WHEN** `add_smart_filter` is called on an eligible smart-object layer with a `filter_id` not already present
- **THEN** it returns `true` and the typed list gains one enabled entry with that id

#### Scenario: Deleting and reordering follow the codec

- **WHEN** `delete_smart_filter` and `reorder_smart_filters` are called on an eligible layer
- **THEN** each returns `true` and the typed list reflects the removal or move

#### Scenario: Clearing empties the stack

- **WHEN** `clear_smart_filters` is called on an eligible layer with filters
- **THEN** it returns `true` and the typed list is empty

#### Scenario: A non-smart layer is refused

- **WHEN** any of the operations is called on a layer with no smart object, a group, or an adjustment layer
- **THEN** it returns `false` and the document is unchanged

#### Scenario: An out-of-range index is refused

- **WHEN** delete or reorder is called with an index outside the typed list
- **THEN** it returns `false` and the document is unchanged

### Requirement: The Clear Smart Filters command records exactly one undo state

The application SHALL expose `Layer > Smart Filter > Clear Smart Filters` with
the stable id `layer.smartFilters.clear`. The command SHALL be enabled only when
the current layer carries at least one smart filter. On dispatch it SHALL invoke
the engine operation on the current layer, recomposite, and record exactly one
history state labelled `"Clear Smart Filters"`. It SHALL record no history state
when disabled or when no filters are present.

#### Scenario: Success records one state and empties the stack

- **WHEN** the command runs on the current layer with one or more smart filters
- **THEN** exactly one history state labelled `"Clear Smart Filters"` is added and the layer reports no smart filters

#### Scenario: Disabled when the layer has no filters

- **WHEN** the current layer is not a smart object or has no smart filters
- **THEN** the command is disabled and records no history state

#### Scenario: No document records nothing

- **WHEN** no document is open
- **THEN** the command is disabled and records no history state

### Requirement: Convert to Smart Object eligibility

The engine operation `pictura-render::convert_to_smart_object(doc, path)` SHALL
convert the layer at `path` only when it is a single raster pixel layer in an
8-bit document: the path resolves, the document is 8-bit, the layer is not a
group, has no adjustment data, has no existing smart object, and has a `rect`
with positive width and height. The Background layer SHALL be eligible. On any
other target the operation SHALL return `false` and SHALL NOT mutate the
document.

#### Scenario: A raster pixel layer converts

- **WHEN** `convert_to_smart_object` is called on the path of a raster pixel layer that has no smart object
- **THEN** it returns `true` and the layer carries an embedded smart object

#### Scenario: A Background layer converts

- **WHEN** `convert_to_smart_object` is called on the Background layer
- **THEN** it returns `true`, the layer carries an embedded smart object, and it is no longer the Background

#### Scenario: A group is refused

- **WHEN** the target path resolves to a group layer
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An adjustment layer is refused

- **WHEN** the target layer has adjustment data
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An existing smart object is refused

- **WHEN** the target layer already has a smart object
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: A zero-size layer is refused

- **WHEN** the target layer's `rect` has zero width or zero height
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: A non-8-bit document is refused

- **WHEN** the document's bit depth is not 8-bit
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An unresolved path is refused

- **WHEN** `path` does not resolve to a layer
- **THEN** the operation returns `false` and the document is unchanged

### Requirement: A Background is unlocked before conversion

When the target of `convert_to_smart_object` is the Background layer, the
operation SHALL first make it an ordinary layer — clear the Background flag,
release its locks, add an opaque alpha channel when it has none, and rename it to
the document's next free `Layer N` — and SHALL then embed its raster as an
embedded smart object. The unlock SHALL be part of the same operation, SHALL NOT
record a separate state, and SHALL NOT change the composite.

#### Scenario: The converted layer is an ordinary layer

- **WHEN** the Background layer is converted
- **THEN** its Background flag is cleared and its locks are released

#### Scenario: A channel-less Background gains an opaque alpha channel

- **WHEN** the Background layer has no alpha channel and is converted
- **THEN** the layer carries an opaque alpha channel

#### Scenario: The composite is unchanged

- **WHEN** the Background layer is converted
- **THEN** sampling the composite before and after returns the same pixels

#### Scenario: The name becomes the next free Layer N

- **WHEN** the Background layer is converted
- **THEN** the layer is renamed to the document's next free `Layer N`

### Requirement: Vector Edit Contents opens an external session

For an editable embedded PDF or PDF-compatible AI, Edit Contents SHALL export
the source to a private temporary file with its format-appropriate extension,
launch the desktop-associated application for that file, and offer Apply and
Discard. Opening the session SHALL add no origin history state. Failure to
launch SHALL report an error and clean up the session files.

#### Scenario: Launch the desktop-associated editor [vector_edit_launch]
- **WHEN** Edit Contents runs on an eligible vector Smart Object
- **THEN** the desktop handoff receives a temporary copy with the exact source bytes and correct extension, Apply and Discard are available, and no editor tab or origin history state is added

#### Scenario: The desktop handler cannot launch [vector_edit_handler]
- **WHEN** the desktop handoff refuses the file
- **THEN** the command reports failure, removes the temporary copy, and records no history

### Requirement: Apply a vector edit to shared instances

Apply SHALL validate and render the temporary source before committing. For a
changed source it SHALL update every instance sharing that embedded source in
the originating document, preserving per-layer transforms, masks, effects,
and Smart Filters. Independent sources SHALL retain their content. Apply
SHALL record one Edit Contents state and Undo and Redo SHALL restore the
matching bytes and preview for every affected layer. An unchanged file, an
invalid or missing page, a discarded edit, or a stale source SHALL record no
state. Failed and stale Apply operations SHALL report the error. Discard and
origin close SHALL release the temporary file.

#### Scenario: Apply after layer reordering [vector_edit_apply]
- **WHEN** the editor saves valid changed artwork and Apply runs after layers have been reordered
- **THEN** the shared source and all its instances update together without affecting an independent source, and Undo and Redo restore the bytes and previews

#### Scenario: Unchanged or failed Apply [vector_edit_refused]
- **WHEN** Apply reads unchanged bytes, invalid bytes, a source without the selected page, or encounters an origin source that was replaced
- **THEN** no history state or artwork change is committed

#### Scenario: Discard or close a vector session [vector_edit_discard]
- **WHEN** the user discards the external edit or closes the originating document
- **THEN** no edit is committed and the session files are removed
