# Design

## Context

The Ruler line lives in `PictureView` and reports pixel geometry through the
annotation bridge. `Document` holds the image-resource bytes and is already
part of each history snapshot. Eager and lazy PSD/PSB open both resolve typed
document metadata after container decoding. The writer already combines typed
edits with preserved image-resource blocks.

## Goals and scope

Give the existing Ruler a calibrated distance, add durable per-document
calibration and reusable presets, and define the output contract for #510.
The Measurement Log, data-point presets, and calculation of selection and Count
statistics belong to #510. Scale markers, DICOM and resource 1080 editing
remain outside this change.

## Decisions

### Document state and history

Keep a validated `MeasurementScale` value in `pictura-core` with explicit units
and pure conversion methods. Represent untouched resource data, an applied
custom value, and an explicit reset distinctly in document state. A file with
no scale and a file with opaque 1074 both display Default, but resetting the
opaque resource changes saved content. The original image-resource section
provides the bytes for preservation and for undoing that reset. The existing
document snapshot gives Undo and Redo the typed value and the edit intent.

Route confirmed scale edits through the existing history and dirty-state
mechanisms. Refresh the calibrated readouts on a scale change, history restore,
or active document switch without recompositing. Preserve the Ruler geometry
and Info panel position/size units as independent inputs. Keep draft values in
the dialog while the user interacts with the canvas. The dialog reads new ruler
geometry through the existing notification and applies scale changes to the
document it opened for. Cancel the draft when that document stops being the
active document or closes. Use document identity. Switching between views of
the same document and changing keyboard focus to the canvas do not cancel it.

The shared successful document-resize path covers Image Size and transform
dispatch. An actual pixel resample resets a recognized custom scale to Default
as part of the resize history state and indicates that recalibration is
required. Undo restores the scale and resource edit intent. Resolution-only
edits, no-op resizes, and failed resizes preserve calibration. Preserve opaque
1074 bytes through an implicit resample, including in Standard. An opaque scale
remains unavailable for measurement and the user is told recalibration is
required. Perspective Crop also invalidates calibration when its mapping changes
distances, including unchanged output dimensions. Rigid transformations preserve
it. A pending notice follows an inactive document and is discarded when
calibration is restored.

### Presets and edition gating

Follow the existing XDG data location for application presets, with a dedicated
measurement-scale store. Validate values on read and write and use the existing
atomic save pattern. Keep names as user data and apply the stored values by
copy, so deleting or changing a preset has no effect on calibrated documents.
Use the existing edition gate for the actions and readouts. Use the same codec
in both editions. Check Default for the default scale. For custom values, check
the matching named preset with the lexicographically smallest name, or Custom
when no preset matches. Match all three entered values exactly, including units.
Equivalent ratios do not match. Preserve the distinction between Default and
an explicitly applied custom identity ratio. Recompute the check on history
and document changes.

### Resource 1074

Reuse `pictura-codec` Action Manager descriptor and resource helpers. Resolve the scale along
both the eager and lazy PSD/PSB paths and use the common PSD/PSB writer path for
edits. Preserve untouched 1074 blocks and unrelated untouched resource bytes,
including trailing bytes. Apply existing writer transformations for other
metadata independently, including guide and ICC edits. When calibration is
edited, change only 1074 in the calibration-specific resource step. For a
recognized descriptor, preserve descriptor fields outside the editable calibration where the parser
can retain them.

The local `measurement-scale-75px.psd` contains a version-16 resource whose
root class is `null`. Its `null` reference addresses the `measurementScale`
property of the target `Dcmn`. The `T   ` item holds a `measurementScale`
object with `pixelLength` (`long`, 75), `logicalLength` (`doub`, 2.5), and
`logicalUnits` (`TEXT`, `µm` followed by a UTF-16 NUL). The codec uses the
observed wrapper and strips the trailing NUL from the effective unit label.

Adobe’s scripting reference documents Pixel Length as a `Long`, Logical
Length as a `Double`, and Logical Units as a `String`. The planned controls use
that range and round raw ruler lengths when filling Pixel Length.
Use the project’s generators for any tracked fixture. [Adobe scripting reference](https://community.adobe.com/havfw69955/attachments/havfw69955/photoshop/556207/1/photoshop-vbs-ref-2020_unlocked.pdf) · [Adobe file format specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/)

### Export inputs and destinations

Place the writers in the existing codec/export layer. Accept a caller-supplied
projection of ordered columns and frozen rows, with histograms already
computed. Keep the writer independent of session state, selection geometry,
and Qt so #510 can serialize the rows displayed in the Log. Use UTF-8 TSV as
the project convention and carry histogram filenames relative to the table.
The workflow spec describes TSV and Adobe’s export instructions describe
comma-delimited output. Verify the actual CS6 convention separately if exact
export interoperability is required. [Adobe export instructions](https://helpx.adobe.com/uk/photoshop/desktop/use-grids-measurement-guides/measure-scale/manage-measurement-logs.html)

Reserve destinations with exclusive creation. On failure, remove only files
and directories owned by that export attempt and return the original error
with any cleanup failure. Inject a write failure in tests so cleanup is
verified without relying on host directory permissions.

## Risks and verification

- Project-generated 1074 fixtures exercise the codec implementation. The
  local sample established the input field layout. The manual Photoshop round
  trip checks that Photoshop accepts the authored output.
- A calibration can be saved even after its application preset is deleted.
  Test undo, preset deletion, tab switching, and PSD/PSB reopen independently.
- The existing non-goal register groups measurement with DICOM. The roadmap
  can record what #635 ships while the DICOM scope remains a separate decision.

## Migration plan

Existing documents without 1074 use Default and existing preset stores need
no migration. Older application versions can continue to re-emit raw 1074.
