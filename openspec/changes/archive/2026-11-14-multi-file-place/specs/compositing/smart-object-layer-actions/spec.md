# Spec Delta

## MODIFIED Requirements

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
recomposite, record exactly one history state labelled `"Place"`, and return the
new layer path. On refusal the bridge SHALL return an empty string, record no
history state, and leave the document unchanged; the command SHALL continue with
the remaining selected paths.

After each successful place the application SHALL select the new layer and start
a Free Transform session on it as defined by the `free-transform` capability.
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
- **THEN** a Free Transform session begins on the new layer and the `"Place"` history state remains recorded

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
