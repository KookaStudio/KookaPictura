## MODIFIED Requirements

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
