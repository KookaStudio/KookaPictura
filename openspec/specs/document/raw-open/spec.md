# document/raw-open Specification

## Purpose
File ▸ Open of a camera raw file: signature routing, the Pictura Raw dialog before the document, seed settings, absolute white balance, and the unsupported-file refusal (#400).

## Requirements

### Requirement: A camera raw file is routed by its signature

Opening a document by path SHALL detect a camera raw container (DNG, CR2 or
NEF) from the file's signature, whatever its extension, and route it to the
raw open before the raster route. A TIFF without `DNGVersion`, a Photoshop
Raw `.raw` file and an unreadable path SHALL NOT be routed to it. The Open
dialog SHALL offer a "Camera Raw (*.DNG *.CR2 *.NEF)" filter, and its All
Formats filter SHALL include all three extensions.

#### Scenario: Raw files are detected by signature [raw_open_sniff]

- **WHEN** a synthetic DNG saved as `shot.tif`, a plain TIFF, and a missing path are tested
- **THEN** only the DNG is detected as camera raw

#### Scenario: The Open dialog lists Camera Raw [raw_open_filter]

- **WHEN** the Open dialog's filters are read
- **THEN** one is "Camera Raw (*.DNG *.CR2 *.NEF)" and All Formats contains `*.DNG`, `*.CR2` and `*.NEF`

### Requirement: An interactive raw open shows Pictura Raw before the document

An interactive raw open SHALL decode the file off the GUI thread, then show the
Pictura Raw dialog on the decoded image before any document opens. OK SHALL
develop the image at full size with the dialog's settings and open a new
16-bit RGB document named after the file, with one opaque Background-style
layer and one "Open" history state. Cancel SHALL open no document and report
the open as cancelled.

#### Scenario: OK opens a 16-bit document [raw_open_dialog_ok]

- **WHEN** a synthetic 16 × 12 DNG is opened interactively and the dialog is accepted
- **THEN** the dialog was shown while no new document existed, and one document opens, 16-bit RGB, 16 × 12, with one layer and one history state

#### Scenario: Cancel opens nothing [raw_open_dialog_cancel]

- **WHEN** the same DNG is opened interactively and the dialog is cancelled
- **THEN** the open reports Cancelled and the document count is unchanged

### Requirement: A non-interactive raw open develops with the seed settings

A non-interactive raw open (control server, command line, self-test, batch)
SHALL NOT show the dialog and SHALL develop with the seed settings.

#### Scenario: Opening without the dialog [raw_open_noninteractive]

- **WHEN** the synthetic DNG is opened non-interactively
- **THEN** no dialog is shown and a 16-bit RGB document of the cropped size opens

#### Scenario: The develop matches the codec at the seed [raw_open_develop_matches_codec]

- **WHEN** the app's develop runs on a decoded source with its seed settings
- **THEN** its 16-bit samples equal `develop_raw` at the as-shot white balance, so the white balance is not applied a second time by the relative stage

### Requirement: The seed settings come from the file

The seed settings SHALL start with Temperature and Tint at the as-shot white
balance, rounded to whole kelvin and tint. The file's settings record SHALL
then supply every control it sets, on every tab, its `Temperature` and `Tint`
taken as absolute, except that a record whose `crs:WhiteBalance` is "As Shot"
SHALL keep the as-shot Temperature and Tint. The record SHALL come from the
preferred store (`document/raw-settings-storage`), else the other store (the
database record named by the file's content hash, or the `<stem>.xmp`
sidecar), else the DNG's embedded XMP.

#### Scenario: As-shot seeds the white balance [raw_open_seed_as_shot]

- **WHEN** a DNG with no sidecar, database record or embedded XMP is decoded
- **THEN** the seed's Temperature and Tint equal the as-shot temperature and tint rounded

#### Scenario: A sidecar overrides the as-shot values [raw_open_seed_sidecar]

- **WHEN** `shot.xmp` beside `shot.dng` sets `crs:Temperature="3200"` and `crs:Exposure2012="+1.00"`, and the DNG embeds `crs:Temperature="4300"`
- **THEN** the seed's Temperature is 3200 and Exposure 1.0

#### Scenario: Embedded settings apply without a sidecar [raw_open_seed_embedded]

- **WHEN** the DNG embeds `crs:Temperature="4300"` and there is no sidecar or database record
- **THEN** the seed's Temperature is 4300

#### Scenario: The preferred store is read first [raw_open_seed_preferred_store]

- **WHEN** the sidecar sets `crs:Exposure2012="+1.00"` and the database record for the same file sets `crs:Exposure2012="-1.00"`
- **THEN** the seed's Exposure is −1.0 with the database preferred and 1.0 with the sidecar preferred

#### Scenario: Other tabs seed from the record [raw_open_seed_all_tabs]

- **WHEN** the sidecar sets `crs:Sharpness="55"` and `crs:SplitToningShadowHue="210"`
- **THEN** the seed's sharpen amount is 55 and its split-toning shadow hue 210

#### Scenario: An As Shot record keeps the as-shot balance [raw_open_seed_as_shot_label]

- **WHEN** the sidecar sets `crs:WhiteBalance="As Shot"` and `crs:Temperature="9000"`
- **THEN** the seed's Temperature is the rounded as-shot temperature

### Requirement: White balance is absolute for a raw source

In the dialog for a raw source, Temperature SHALL run 2000–50000 K on a
reciprocal (mired) scale and Tint −150…+150, starting at the seed values, and
the Basic panel's reset SHALL return them to the as-shot values. The preview
and the develop SHALL apply them in the colour stage. A layer source SHALL keep
the relative −100…+100 scale.

#### Scenario: Raw mode shows absolute sliders [raw_open_absolute_sliders]

- **WHEN** the dialog opens for a raw source seeded at 5500 K, and for a layer
- **THEN** the raw dialog's Temperature runs 2000–50000 starting at 5500 and Tint −150…150, and the layer dialog's run −100…100

#### Scenario: The reciprocal slider spreads daylight [raw_open_reciprocal_slider]

- **WHEN** a reciprocal 2000–50000 slider's handle is placed at the middle of its track
- **THEN** its value is between 3500 and 4500 K

#### Scenario: The preview follows the temperature [raw_open_temperature_preview]

- **WHEN** the raw preview renders with Temperature 3000 and 9000
- **THEN** the 9000 K preview's mean red-to-blue ratio is greater

### Requirement: An unsupported raw file fails clearly

A raw open whose file the codec refuses SHALL open no document, SHALL report
"Could not open <name>: <reason>" with the codec's reason, and SHALL NOT crash.

#### Scenario: A file without camera colour is refused [raw_open_unsupported]

- **WHEN** a synthetic DNG without `ColorMatrix1` is opened non-interactively
- **THEN** the open fails, no document is added, and the status message names the file and says the camera colour is not supported
