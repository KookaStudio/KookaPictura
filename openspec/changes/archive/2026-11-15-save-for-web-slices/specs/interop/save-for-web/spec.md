# Spec Delta

## MODIFIED Requirements

### Requirement: Web palettes and encoders

The system SHALL reduce an image to an indexed palette by a Color Reduction
method (Perceptual, Selective, Adaptive, Restrictive, Black and White,
Grayscale) within 2–256 colours, optionally dithered (Diffusion, Pattern,
Noise, with an amount), keeping transparency over a matte when asked, and SHALL
encode it as GIF89a (optionally interlaced) or WBMP that independent decoders
read exactly. It SHALL also reduce an image to a caller-supplied palette: a
Custom table from a loaded `.act`, or the active reduction augmented with
locked colours that SHALL survive the reduction. The effective colour budget
SHALL be `max(Colors, locked.len())` capped at 256, so adding a locked colour
raises the table size up to 256 and adding one when 256 colours are already
locked SHALL be refused. With Transparency on, clear pixels SHALL map to the
table's transparent entry when it has one; with Transparency off, or when the
table has no transparent entry, every pixel SHALL map to its nearest entry
after any partial alpha is laid over the matte. A custom or edited table's
transparency entry SHALL encode as transparent.

#### Scenario: ImageMagick reads our GIF and WBMP

- **WHEN** the `web_oracle` test reduces a colour field with a clear corner to 64 Selective colours with diffusion, encodes it as GIF (plain and interlaced) and WBMP, and decodes them with ImageMagick
- **THEN** every GIF pixel is its palette colour or transparent where clear, and every WBMP pixel is the black or white its bit says

#### Scenario: A locked colour survives reduction [sfw-custom-palette]

- **WHEN** a 64-colour reduction with diffusion requests 16 colours and locks one colour not present in the reduced table
- **THEN** the output palette contains the locked colour, has at most 16 entries, and every pixel uses one of its entries

#### Scenario: Locked colours raise the budget [sfw-color-budget]

- **WHEN** a 16-colour reduction locks 20 colours and, separately, a table already holding 256 locked colours adds one more
- **THEN** the first output table has 20 entries and keeps every locked colour, and the second add is refused

### Requirement: Save for Web & Devices dialog

File > Save for Web & Devices (Ctrl+Alt+Shift+S) SHALL open a dialog on the
flattened document offering GIF, JPEG, PNG-8, PNG-24, and WBMP with their
settings, CS6's presets, Image Size, and Original / Optimized / 2-Up / 4-Up
previews showing the optimised file's size and download time. The dialog SHALL
also show a Color Table for GIF and PNG-8 and a Metadata menu. Save SHALL write
the optimised file, or open Save Optimized As when the document has user
slices. Done SHALL keep the active pane's format and settings as the next
default Optimized pane and SHALL keep the Metadata choice; the other 4-Up
variants SHALL NOT be persisted.

#### Scenario: Optimise, resize, save, remember

- **WHEN** the `tst_save_for_web` test opens the dialog, reads the default GIF, switches formats and settings, halves the size, picks JPEG Low, saves, chooses PNG-24, clicks Done, and reopens it
- **THEN** the GIF decodes with its clear corner, JPEG at quality 10 is smaller than at 100, PNG-8 keeps to its colour table, PNG-24 without transparency lays the corner over white, WBMP has its header, the image is 32 x 24, the preset sets JPEG at 10, the saved file is the optimised bytes, and the reopened dialog shows PNG-24

#### Scenario: The new surfaces appear for their formats [sfw-dialog-surfaces]

- **WHEN** the dialog is opened and GIF, PNG-8, JPEG, and PNG-24 are selected in turn
- **THEN** the Color Table is present only for GIF and PNG-8, the Metadata menu is present for every format, and all four preview tabs are enabled

## ADDED Requirements

### Requirement: Four-pane comparison

The 4-Up tab SHALL show four panes: the original top-left, and three optimised
panes, each of which SHALL keep its own format and settings and SHALL annotate
its format, optimised file size, and 56.6 Kbps download time. Clicking an
optimised pane SHALL make it the active pane, so the option controls read and
edit that pane's settings, and Save SHALL use the active pane's settings. The
original pane SHALL NOT be editable. Repopulate Views SHALL regenerate the
three optimised panes from the active pane's settings with progressively lower
quality or smaller colour tables.

#### Scenario: Selecting a pane edits only that pane [sfw-four-up]

- **WHEN** the 4-Up tab is shown, the second optimised pane is clicked, and its JPEG quality is lowered
- **THEN** that pane's annotation reports a smaller file than before, the other optimised panes are unchanged, and Save writes the lowered-quality JPEG

#### Scenario: Repopulate lowers the other panes [sfw-repopulate]

- **WHEN** Repopulate Views is invoked with GIF 128 colours active
- **THEN** the three optimised panes are regenerated with no more than 128 colours each and sizes no larger than the active pane's

### Requirement: Colour-table editing

For GIF and PNG-8 the dialog SHALL show the active pane's colour table as
swatches and SHALL allow the user to select a swatch, shift its colour, add a
locked colour, delete a colour, sort the table by hue, luminance, or
popularity, map or unmap the selected colour to transparency, and load or save
a `.act` table. An edited table SHALL be the table the GIF or PNG-8 encoder
writes. Adding a colour SHALL lock it so it survives a later reduction. Deleting
a colour SHALL remap the pixels that used it to their nearest remaining entry.
Mapping to transparency SHALL leave exactly one transparent entry, and
unmapping that entry SHALL leave it in the table as an opaque colour at its RGB.
Loading an `.act` SHALL use it as a Custom table and map every pixel to its
nearest entry. A `.act` file of 768 or 772 bytes SHALL load, with a 772-byte
file truncated to its count. Saving SHALL write
the 772-byte form with a 2-byte big-endian count and a 2-byte big-endian
transparent index when the table has fewer than 256 entries or a transparency
entry, and the 768-byte form only for a full 256-entry opaque table, so a
16-colour table SHALL round-trip its colours, size, and transparency entry.

#### Scenario: Edit and encode through the table [sfw-color-table]

- **WHEN** a GIF is reduced to 16 colours, one swatch is shifted to red, and the GIF is saved
- **THEN** the decoded GIF uses the edited red entry for the pixels that used that swatch

#### Scenario: Map a colour to transparency [sfw-color-transparency]

- **WHEN** a swatch is mapped to transparency and the GIF is saved
- **THEN** the decoded GIF's pixels using that swatch are transparent and the table has one transparent entry

#### Scenario: Unmapping makes the entry opaque [sfw-color-unmap]

- **WHEN** a swatch is mapped to transparency, then unmapped, and the GIF is saved
- **THEN** the entry stays in the table at its RGB, no entry is transparent, and the pixels using it decode opaque

#### Scenario: Sort and act round-trip [sfw-color-sort]

- **WHEN** the table is sorted by luminance and saved as `.act`, then loaded again
- **THEN** the loaded colours match the sorted table in order and the image still encodes with the same visible result

### Requirement: Metadata choice

The dialog SHALL offer a Metadata menu with None, Copyright, Copyright and
Contact Info, All Except Camera Info, and All. None SHALL be the default and the
choice SHALL be kept for the next opening in the session. The menu SHALL NOT
change the document or its optimised pixels. For JPEG, None SHALL write no
EXIF/IPTC/XMP (except a minimal EXIF carrying only the original copyright notice
when one exists); Copyright SHALL write the copyright notice; Copyright and
Contact Info SHALL add the creator, credit, and contact fields; All Except
Camera Info SHALL write the original IPTC and XMP without EXIF; All SHALL write
the original EXIF, IPTC, and XMP. PNG-8 and PNG-24 SHALL carry the selected XMP
packet when the document has one (the generated copyright packet for the
Copyright levels), and SHALL write no metadata for None; GIF and WBMP SHALL
carry none.

#### Scenario: All carries the blocks, None strips them [sfw-metadata]

- **WHEN** a document with EXIF, IPTC, and XMP is saved twice as JPEG, once with Metadata All and once with None
- **THEN** the All file carries EXIF APP1, XMP APP1, and the IPTC Photoshop APP13 block, and the None file carries none of them, keeping at most a minimal EXIF copyright

#### Scenario: PNG carries XMP but GIF does not [sfw-metadata-formats]

- **WHEN** the same document is saved with Metadata All as PNG-24 and as GIF
- **THEN** the PNG contains an XMP `iTXt` chunk and the GIF contains no metadata block

#### Scenario: The choice is remembered [sfw-metadata-remember]

- **WHEN** Metadata All is chosen, the dialog is closed with Done, and reopened
- **THEN** the Metadata menu shows All
