# codec/raw-settings-xmp Specification

## Purpose
Reading and writing the Pictura Raw settings as Camera Raw `crs:` XMP, patching an existing packet in place, and the content-hash key of a raw file, for raw settings storage (#401).

## Requirements

### Requirement: Every Pictura Raw control reads from its crs name

`pictura_raw_from_xmp` SHALL read every control of the Pictura Raw settings
model that has a Camera Raw `crs:` property — the Basic controls, process
version, grayscale, white-balance preset, the parametric tone curve and its
splits, the four PV2012 point curves, Detail, the HSL and grayscale-mix
ranges, Split Toning, Lens, Effects, and Camera Calibration — from attribute
or element form. A missing or unparsable property SHALL leave its control
unset. `ProcessVersion` 6.7, 5.7 and 5.0 SHALL read as 2012, 2010 and 2003,
and the `WhiteBalance` labels as the dialog's preset codes.

#### Scenario: Controls from several tabs are read [raw_xmp_reads_tabs]

- **WHEN** a packet sets `crs:Exposure2012="+0.50"`, `crs:Sharpness="40"`, `crs:HueAdjustmentOrange="-12"`, `crs:SplitToningBalance="+20"`, `crs:PostCropVignetteAmount="-30"`, `crs:CameraProfile="Camera Matching"`, `crs:ConvertToGrayscale="True"`, `crs:ProcessVersion="6.7"`, `crs:WhiteBalance="Custom"`, and a `crs:ToneCurvePV2012` sequence `0, 0` / `128, 140` / `255, 255`
- **THEN** the settings hold exposure 0.5, sharpen amount 40, orange hue −12, split-toning balance 20, post-crop vignette −30, camera profile "Camera Matching", grayscale on, process version 2012, white balance `Cst `, and the RGB curve `[0, 0, 128, 140, 255, 255]`

#### Scenario: A packet without crs reads as unset [raw_xmp_reads_nothing]

- **WHEN** a packet carries only `dc:` properties
- **THEN** the settings equal the default settings

### Requirement: Writing patches a packet and keeps every other byte

`pictura_raw_to_xmp` SHALL render every set control under its `crs:` name and
set `crs:HasSettings="True"`. Given an existing packet it SHALL replace the
text of each modelled property that exists, insert each set one that does
not, and remove each modelled one the settings leave unset, keeping every
other byte of the packet — unknown `crs:` properties, other namespaces, and
the wrapper — in place. Without a packet it SHALL build a fresh XMP packet.
It SHALL return nothing for a packet it cannot patch (no `rdf:RDF` element).
Reading what it wrote SHALL give back the written settings.

#### Scenario: Settings round-trip through a fresh packet [raw_xmp_round_trip]

- **WHEN** settings with controls set on every tab, including all four point curves and a text value holding `&` and `<`, are written without a packet and read back
- **THEN** the settings read back equal the written settings

#### Scenario: Unknown keys survive a rewrite [raw_xmp_preserves_unknown]

- **WHEN** a sidecar holding `crs:Exposure2012="+1.00"`, `crs:HasSettings="True"`, an unknown `crs:LensProfileName`, a `crs:RetouchInfo` element sequence, and a `dc:title` is written with exposure 0.25
- **THEN** the result equals the sidecar with only `+1.00` replaced by `+0.25`

#### Scenario: Unset controls are removed and curves inserted [raw_xmp_removes_unset]

- **WHEN** a self-closing description holding `crs:Contrast2012="+25"` is written with settings that leave contrast unset and set an RGB point curve
- **THEN** the packet no longer holds `Contrast2012`, holds a `crs:ToneCurvePV2012` sequence inside the description, and reads back the curve

#### Scenario: A packet without rdf:RDF is refused [raw_xmp_refuses_unpatchable]

- **WHEN** the existing packet is `not xmp`
- **THEN** `pictura_raw_to_xmp` returns nothing

### Requirement: The database key is the file's content hash

`raw_settings_key` SHALL return the lowercase hexadecimal SHA-256 of the file's
bytes.

#### Scenario: The key is the SHA-256 [raw_settings_key_sha256]

- **WHEN** the key of the bytes `abc` is taken
- **THEN** it is `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`
