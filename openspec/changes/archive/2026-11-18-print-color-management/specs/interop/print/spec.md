# Print colour management delta

## MODIFIED Requirements

### Requirement: Print dialog

File > Print… (Ctrl+P) SHALL open a dialog that previews the flattened image
centred and fitted on the destination's page in the chosen orientation, offers
every available printer and Save as PDF, and prints the image scaled to fit the
printable area with the chosen copies and orientation. The preview and print
SHALL share the placement and mark layout. Done and successful Print SHALL
commit settings as defaults for the next Print and for Print One Copy. Cancel,
closing the dialog, cancelling the PDF file chooser, and rejected or failed
submission SHALL leave the defaults unchanged. Managed CUPS Print succeeds when
submission returns success and a job ID; printer-managed Qt Print succeeds when
the painter completes successfully, and PDF Print succeeds on file publication.
A later physical failure after acceptance SHALL NOT automatically roll back
defaults.

#### Scenario: Preview, orientation, and a PDF job

- **WHEN** a user opens Print on a red image, toggles landscape and Show Paper White, cancels, then sets two copies and landscape and prints to Save as PDF
- **THEN** Save as PDF is listed last, the Letter page reads 8.5 in x 11 in portrait with the image centred and 11 in x 8.5 in landscape, the paper tints, Cancel keeps the previous defaults, and Print writes a PDF and keeps two copies and landscape as the defaults. Cancelling the file choice writes no PDF.

### Requirement: Print One Copy

File > Print One Copy SHALL submit one copy using the active document's
resolved print settings without a dialog when the last destination is an
available and eligible native printer and preflight succeeds. Otherwise it SHALL
open the Print dialog; a blocked preflight SHALL explain the reason there. The one-copy
override SHALL leave the saved copy count unchanged. Both commands SHALL be
enabled only with a document.

#### Scenario: The commands follow the document

- **WHEN** File > Print… and Print One Copy are inspected before and after creating a document
- **THEN** Print… carries Ctrl+P, both commands are disabled without a document and enabled with one, and the printable image is the flattened document.

#### Scenario: One Copy uses the accepted job settings [print_one_copy_settings]

- **WHEN** a document with saved colour, proof, and mark settings has an available, supported native destination, a valid output profile when required, and the last copy count is three
- **THEN** Print One Copy sends that document's job with one copy and leaves the saved count at three. When the destination is Save as PDF or unavailable, the command opens Print. A preflight-blocked queue opens Print with an actionable reason.

### Requirement: Print Settings page setup reaches the preview and the job

Print Settings SHALL retain the chosen page size and paper source together with
the printer, copies, and orientation. The dialog preview and page-size label
SHALL use the retained page size. Print and Print One Copy SHALL apply the
retained page size and paper source to native jobs. Save as PDF SHALL apply the
page size to the PDF MediaBox; PDF has no paper source. Print Settings SHALL
be available for Save as PDF. Done and successful Print SHALL persist the page setup; Cancel SHALL not. The page-setup
dialog SHALL be replaceable by a test provider.

#### Scenario: A4 chosen in Print Settings reaches the preview and the job

- **WHEN** a test page-setup provider selects A4 and Cassette and the dialog is accepted, then a PDF and native job are prepared
- **THEN** the label and preview use A4, settings retain A4 and Cassette, the emitted PDF MediaBox has A4 dimensions, and the native submission receives the selected page size and source. The existing printer-managed Qt path also applies these choices.

#### Scenario: Physical tray follows Print Settings [print_page_setup_physical]

- **WHEN** A4 and Cassette are selected and a printer-managed job is submitted to a real CUPS queue
- **THEN** the physical job uses the selected paper source.

## ADDED Requirements

### Requirement: Print colour handling

The Print dialog SHALL show the document profile and default to Printer
Manages Colors, which delegates conversion to the destination and applies no
printer-profile transform. Application-managed colour SHALL let the user select
an installed or explicitly opened Gray, RGB, or CMYK printer ICC profile, one
of the four ICC rendering intents, and black-point compensation. The output
SHALL use the selected profile's device samples and identify their colour
space. Save as PDF SHALL carry the ICC profile. The dialog SHALL explain the
driver setup needed to avoid a second conversion. An unavailable or unusable
profile or an unsupported native submission SHALL produce an actionable error
and SHALL prevent submission in that mode.

Managed native CUPS submission SHALL require an explicit per-destination
administrator or user configuration for a known colour-management bypass
route, validated against the same device, queue, filter chain, media, output
device colour space and profile, and relevant job colour options. PDF and option
support alone SHALL NOT establish eligibility. Unknown, observably changed, or
unverifiable configurations SHALL be rejected before submission with an explanation and a choice of Save as PDF or Printer Manages
Colors. The opt-in SHALL record route validation evidence and identify
observable configuration changes; external administrator changes that cannot be observed
SHALL invalidate the opt-in and require the administrator or user to revoke and
revalidate it. Submission failures reported after an eligible attempt SHALL be
reported separately and SHALL leave accepted defaults unchanged.

#### Scenario: Application-managed PDF carries the device conversion [print_managed_pdf]

- **WHEN** an image is printed to PDF with application-managed colour and a printer ICC profile, and again with Printer Manages Colors
- **THEN** the managed image has the selected ICC profile and device samples matching the reference transform with the chosen intent and black-point compensation. The printer-managed image has no target-profile conversion.

#### Scenario: Missing or invalid printer profile [print_invalid_profile]

- **WHEN** application-managed printing is requested with an unavailable or unusable profile
- **THEN** the dialog identifies the problem, retains the selection for correction, and writes or submits no job.

#### Scenario: Managed CUPS eligibility and submission [print_managed_cups]

- **WHEN** a managed job is prepared for a validated queue and matching device, filter and media configuration, and then for an unknown or changed configuration
- **THEN** only the matching configuration is eligible, the prepared submission carries the chosen page size, source, orientation and copies, and unknown or changed configurations are rejected before queuing with alternatives. A submission failure is reported and leaves defaults unchanged; success with a job ID commits defaults without automatically rolling them back on later physical failure.

#### Scenario: Managed CUPS physical bypass [print_managed_cups_physical]

- **WHEN** a known target is submitted to an explicitly opted-in queue with its validated device, filter chain and media configuration
- **THEN** inspection of device samples or equivalent raster diagnostics confirms the bypass for that configuration, and the physical job uses the chosen options.

### Requirement: Print preview and hard proofing

Match Print Colors SHALL preview the selected printer profile in
application-managed mode. Gamut Warning SHALL be available with Match Print
Colors and mark out-of-gamut pixels in the preview. Show Paper White SHALL
change the preview paper appearance, using the profile when available. These
preview controls SHALL leave the printed raster and marks unchanged.
Hard Proofing SHALL require application-managed colour and a valid output ICC
profile. In Printer Manages Colors, the hard-proof control SHALL be unavailable
with an explanation; switching modes SHALL retain any saved proof setup without
submitting it. A missing or invalid output profile SHALL block hard-proof output.
A saved document print proof setup SHALL take precedence on reopening; when no
such setup exists, Hard Proofing SHALL initialize from the active View proof
setup whether or not canvas Proof Colors is on. It SHALL permit a print-only
proof setup override and use the proof condition and selected printer profile
for the preview and actual output. Simulate Paper Color and Simulate Black Ink
SHALL affect the proof transform when supported. A missing proof profile SHALL show an unproofed
preview with an explanation and prevent a hard-proof job until corrected.

#### Scenario: Preview aids are preview-only [print_preview_aids]

- **WHEN** Match Print Colors, Gamut Warning, and Show Paper White are toggled on an image with known in-gamut and out-of-gamut patches
- **THEN** the preview shows the destination appearance, the warning marks the out-of-gamut patch, and the paper appearance changes. The PDF samples are identical with the preview aids on and off.

#### Scenario: Hard proof simulates the press in output [print_hard_proof]

- **WHEN** Hard Proofing is selected with canvas Proof Colors off and a valid View proof setup, then an independent print override or simulation flag is chosen
- **THEN** the preview and emitted printer-device samples follow the selected proof settings, and the active View setup, canvas state, document pixels, and history are unchanged. An unavailable proof profile produces an explained unproofed preview and blocks the hard-proof job.

#### Scenario: Hard proof mode and saved setup [print_hard_proof_mode]

- **WHEN** a document with a saved print proof setup is reopened with a different active View proof, then Printer Manages Colors is selected or the output ICC is missing or invalid
- **THEN** the saved print setup takes precedence and remains retained; the hard-proof control is unavailable with an explanation in printer-managed mode, and missing or invalid output ICC blocks a hard-proof job. A document without a saved setup initializes from View.

### Requirement: Print output marks

The Output section SHALL offer calibration bars, registration marks, corner
and centre crop marks, filename labels, File Info description, page background,
border width, and bleed width. The preview and emitted job SHALL use the same
positions and physical sizes for those decorations. Bleed SHALL offset the
crop marks inward from the image bounds. Marks outside the image SHALL be
suppressed where the printable paper has insufficient space. When marks fit,
the image SHALL remain centred in the printable area after reserving their
space.

#### Scenario: Page marks appear in preview and PDF [print_marks_layout]

- **WHEN** output decorations are enabled on a page with room for them and the job is saved as PDF
- **THEN** the preview and PDF agree on image placement, mark positions, border, background, and bleed. The filename and File Info description appear in the selected locations.

#### Scenario: Marks are suppressed where they cannot fit [print_marks_no_room]

- **WHEN** the printable bounds cannot fit a mark outside the image
- **THEN** that mark is absent from preview and output and the image stays within the printable area.

### Requirement: Persisted print defaults

The application SHALL persist accepted defaults across restarts. Portable
print settings read from the active document SHALL take precedence over the
corresponding application defaults. The selected queue, paper source, copies,
and other driver-specific settings SHALL be machine-local. Done or successful
Print SHALL prepare changed portable settings for the document's next PSD/PSB
save. This metadata change SHALL leave pixels, layers, and image edit history
unchanged. Opening a file and inspecting or cancelling Print SHALL preserve the
original print resources.

#### Scenario: Accepted settings persist at the right scope [print_persist_defaults]

- **WHEN** a user accepts changed settings in Print, saves the PSD or PSB, restarts Pictura, and opens it alongside a document with different portable settings
- **THEN** machine-local defaults survive the restart, each document resolves its own saved settings, and accepting print metadata creates no image history entry. Cancellation and failed jobs leave the previous settings and save state intact.

### Requirement: Adobe print resource interoperability

PSD and PSB saves SHALL write supported portable colour, proof, and output-mark
settings using standard Photoshop print resources. The codec SHALL read those
settings in Adobe-authored files and preserve unrelated image resources,
unknown print descriptor entries, and opaque OS-specific print data through
incidental saves and supported print-setting edits. An undecodable resource
SHALL be preserved and diagnosed before any edit that would overwrite it.
File > Print in Photoshop CS6 and the tested Photoshop CC build SHALL restore
at least colour handling, an installed RGB printer profile, rendering intent,
black-point compensation, hard-proof mode and its proof setup, and the marks
offered by both Photoshop versions. Both apps SHALL have the same profiles
installed for the check. The verified field matrix SHALL identify the results
for the remaining controls, including CMYK printer-profile selection and each
proof simulation flag.

#### Scenario: PSD and PSB print resources round-trip [print_codec_round_trip]

- **WHEN** project-generated PSD and PSB files include standard print settings, unknown descriptor entries, opaque platform payloads, and unrelated resources, and a supported setting is edited and saved
- **THEN** the edit is stored under the standard resource and descriptor identifiers, previously unedited fields retain their values, and opaque and unrelated bytes survive. A malformed print resource survives an incidental save and is diagnosed before an edit would replace it.

#### Scenario: Photoshop CS6 and CC restore and resave settings [print_adobe_round_trip]

- **WHEN** a Pictura-saved PSD and PSB with distinctive colour, hard-proof, and mark settings are opened in Photoshop CS6 and the recorded CC build, and each application changes one supported setting and saves a copy
- **THEN** File > Print in each Photoshop version restores the portable fields, Pictura loads the edited value and the other settings from each copy, and a subsequent Pictura save reopens with those values in the same Photoshop version. The result records the build and which fields were accepted.
