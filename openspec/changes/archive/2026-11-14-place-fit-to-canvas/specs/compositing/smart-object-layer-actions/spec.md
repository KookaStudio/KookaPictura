# Spec Delta

## MODIFIED Requirements

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
