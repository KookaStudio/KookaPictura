# Design

## Context

`print_dialog.cpp` gets flattened sRGB through the web-export bridge and paints
through `QPrinter`. The preview computes its fit separately from the print
job. `PrintSettings` currently holds machine-local settings in a process
static. The Rust picture code already provides banded composite reads, and
`pictura-color` and the Photoshop PDF writer already handle Little CMS and
ICC-tagged device samples respectively. See [the proposal](proposal.md) and
[the print delta](specs/interop/print/spec.md) for scope and behaviour.

## Goals / Non-Goals

- Keep layout, colour conversion, and submission independently testable. Keep
  the printer and profile environment outside the deterministic tests.
- Keep the existing centred-fit workflow. Manual positioning, selected-area
  printing, separations, film options, vector/PostScript output, and the macOS
  16-bit driver path are separate work.

## Decisions

### Prepare a page once and use its geometry at each destination

Resolve the selected `QPageLayout` into oriented paper and printable rectangles
in points. Compute the centred-fit image rectangle and mark geometry in those
units. Reserve the required mark gutters when they leave a usable image area
and hide decorations whose footprints cannot fit. Measure border and bleed in
points. Use representative prepress geometry and document it in the tests.
Derive filename and description from the document and its File Info metadata.
The preview scales this same layout to its viewport, and the output applies
the page coordinates. This also keeps orientation and destination margins in
agreement.

Use a reduced composite for interactive previews and the banded picture source
for output. Preserve the source profile and native samples where the current
composite can supply them. Otherwise use its explicitly identified working
projection. Resolve transparency against the intended page background before
encoding an opaque managed device raster so the conversion sees the correct
colour. Keep the existing raster print depth and limit temporary allocations
to bands and output buffers. Draw decorations as page elements so they stay
anchored to the layout without a full-resolution page-sized overlay.

### Distinguish document, proof, display, and output profiles

Use Little CMS with the actual source and destination colour spaces. Resolve
Gray, RGB, or CMYK output ICC files from local installations or explicit file
selection. Retain a stable profile identifier and display name. Resolve an
Adobe profile name against installed profiles and report a missing or ambiguous
match. The default remains Printer Manages Colors.

Use the existing soft-proof conventions for the preview and an ICC gamut check
for its warning. Paper-white preview follows the output profile where available
and keeps the existing neutral paper tint when no profile is selected. For a
hard proof, explicitly route source colours through the press simulation to
the actual printer profile. The existing `ProofPreview` returns sRGB and is
the display path. Hard proof output needs printer-device samples. Reuse the
View proof settings and dialog fields for a local copy, including intent and
simulation flags. A saved document print proof takes precedence over View on
reopen; View initializes only documents without one. Hard proof requires
application-managed colour and a usable output ICC. Disable its control with
an explanation in printer-managed mode, retaining its setup when modes change;
block output if the output profile becomes invalid. A display transform derives
the preview from the configured simulation without pushing print choices back
to View.

### Deliver the managed PDF and the native job through explicit paths

Reuse the codec's ICC image-stream and PDF object primitives for the managed
print page, sized by paper geometry. The Photoshop PDF save entry point sizes
its page by document dimensions and has editing-payload options, so give the
print page a focused entry point. Test the actual emitted image data and ICC
object. Qt's ordinary PDF image path emits device colour-space names, and its
PDF output-intent support varies by PDF version. Those behaviours alone do
not establish that the selected profile reaches the output. [Qt PDF engine](https://raw.githubusercontent.com/qt/qtbase/6.11/src/gui/painting/qpdf.cpp)

Retain Qt Print Support for destination discovery, page setup, and legacy
printer-managed submission. For a managed native job, submit the prepared PDF
as a PDF job without repainting it through `QPrinter`. Inspect the queue's
supported job format and options and pass copies, paper size, source and
orientation through the available CUPS submission path. Eligibility requires
explicit administrator or user setup of a known bypass route for the
destination, validated against the same device, queue, filter chain, media,
output device colour space and profile, and relevant job colour options. The
opt-in records evidence of that route; PDF and option support alone do not
establish a bypass. Compare a visible snapshot of server
and queue ID, available identity, configuration, driver revision and provider
attributes, media, source, output profile and colour space, and job options.
Reject unknown, observably changed or unverifiable configurations before queuing, with PDF and printer-managed
alternatives. External changes that cannot be observed invalidate the opt-in;
the administrator or user must revoke and revalidate it after such changes.
Distinguish preflight rejection from submission failure. Successful managed
CUPS submission returns a job ID and commits defaults. The printer-managed Qt
path uses successful painter completion; PDF uses successful file publication.
Subsequent physical failure does not automatically roll back accepted defaults.
If One Copy fails preflight, open Print with an actionable reason. Test orientation once so the oriented
PDF is not rotated twice. Record queue, device, filter, media, driver settings
and validation method, and inspect device samples or raster diagnostics.
[CUPS destination and job APIs](https://openprinting.github.io/cups/doc/cupspm.html)

On Linux the managed route uses the optional CUPS client commands `lpstat`,
`lpoptions`, and `lp`. The app records the validated route, including its job
options and captured queue fingerprint, under local state. The route dialog
accepts the operator's evidence and supports revocation. The client commands
submit the prepared PDF and return a job ID without a mandatory build dependency.

### Store print defaults at the scope where they apply

Use the app settings store for last accepted defaults, including queue, tray,
copies, page size and orientation. Overlay known per-document portable fields
on those defaults. Profile paths are local references and the Adobe resources
carry portable profile descriptions. Only Done or successful Print applies the
complete settings. Prepare and validate resource changes before submission so
a successful job can commit its settings. Print One Copy resolves settings via
the same path and overrides copies for the submission only.

Keep accepted portable edits in document-session metadata separate from image
undo snapshots. Apply the accepted resource update to the document snapshot
used by native saves. Track metadata changes in the saved/dirty calculation so
undoing an image edit cannot discard them and a concurrent save cannot clear
a newer print edit. Save a Copy includes the accepted settings and leaves the
original save state intact. A no-op acceptance keeps the existing resource
bytes. The app defaults remain available for documents with no print values.

### Write standard PSD/PSB resources and verify the field mapping

Use the codec's image resource framing and ActionDescriptor reader/writer for
version-16 resources 1082 and 1083. Treat 1082 as the source for portable
colour and proof fields, and 1083 as the source for marks. Recognize documented
legacy mark flags where they overlap, with modern values taking precedence.
Synchronize overlapping legacy mark fields on edits where the encoding is
verified. Preserve scale and placement data outside this change's controls and
keep resources 1071, 1084 and 1085 opaque. Patch only the edited descriptor
entries. Carry unknown entries and unrelated resource bytes through the write,
and refuse to overwrite a block the codec cannot decode safely.

Use public mappings and lawful observation of the Print dialog to settle the
class IDs, value types and enum values. Verify the minimum exchange set with
both versions of Photoshop: colour handling, an installed RGB printer profile,
intent, black-point compensation, hard-proof mode and a restorable proof
setup, and the marks exposed by both Adobe versions. Check Simulate Black Ink,
CMYK printer-profile selection, and simulation flags individually. The public
mapping covers RGB and Gray printer selector values and does not establish
the wire form for a CMYK destination selector or a separate black-ink flag.
Record the tested support for each field. Use generated fixtures in CI and
scratch files kept outside the repo for manual Adobe observations.
[Adobe PSD specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/)
· [Public 1082 mapping](https://github.com/Agamnentzar/ag-psd/blob/387049670cb89b88fb8fe1b7c01aeacf98dd2e3b/src/imageResources.ts#L127-L222)
· [Public 1083 mapping](https://github.com/Agamnentzar/ag-psd/blob/master/src/imageResources.ts#L280-L372)
· [Fixture provenance](../../../CONTRIBUTING.md)

## Risks / Trade-offs

- Different Photoshop builds or driver environments can resolve a profile name
  differently. Use the same installed profile in both tests, document OS and
  application versions, and run a missing-profile check in Pictura.
- CUPS may convert a valid managed PDF through a queue filter. Confirm the job
  on a queue configured for application-managed colour before claiming physical
  output support. Reject managed native submission when the supported path is
  unavailable.
- Exact Adobe mark geometry and paper/ink simulation need a measured reference.
  Verify relative geometry, physical sizes and Little CMS samples in automated
  tests. Record the results of the separate Adobe file exchange.

## Migration Plan

Start with existing printer-managed defaults when app settings or document
resources are absent. Load recognizable values from existing files, preserve
opaque print data, and write supported settings when they are edited and the
document is saved. Older Pictura builds can read the PSD/PSB through their
existing image resource handling.
