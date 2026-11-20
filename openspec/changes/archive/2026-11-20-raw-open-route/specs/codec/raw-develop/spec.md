## ADDED Requirements

### Requirement: A raw source is decoded once and refused early when its colour cannot be mapped

`decode_raw_source` SHALL read any container `read_raw` supports, build the
as-shot camera transform from its colour tags before demosaicing, and only then
demosaic with AHD. It SHALL keep the camera RGB, the colour tags, the default
crop, the orientation, the make and model, and the as-shot white balance as
temperature and tint. A file whose colour tags cannot be mapped SHALL be
refused with `PsdError::Unsupported` whose message says the camera colour is
not supported and names the camera when the file names it. Bytes that are not
a supported raw container SHALL be refused with `PsdError::Unsupported`.

#### Scenario: A DNG decodes to a raw source [raw_source_decodes]

- **WHEN** a synthetic 16 × 12 uncompressed DNG with an RGGB pattern, `ColorMatrix1`, `AsShotNeutral` and a default crop is decoded
- **THEN** the source's camera RGB is 16 × 12, its crop and orientation equal the file's, and its as-shot white balance equals `as_shot_transform`'s

#### Scenario: A file without a colour matrix is refused before the demosaic [raw_source_refuses_colour]

- **WHEN** a synthetic DNG without `ColorMatrix1` and with `UniqueCameraModel` "Test Cam", and a synthetic CR2, are decoded
- **THEN** each is refused as `Unsupported`, and the DNG's message contains "camera colour" and "Test Cam"

#### Scenario: Non-raw bytes are refused [raw_source_refuses_non_raw]

- **WHEN** a plain TIFF without `DNGVersion` and an empty buffer are decoded
- **THEN** each is refused as `Unsupported`

### Requirement: The develop turns camera RGB into cropped, oriented 16-bit working RGB

`develop_raw` SHALL take camera RGB, white-balance it at the chosen
temperature and tint and convert it to linear ProPhoto (`white_balance_transform`
and `to_pro_photo`), scale it by `2^BaselineExposure` with clipping, apply the
default crop rounded to whole pixels and clamped to the image, apply the TIFF
orientation (1–8), and convert linear ProPhoto to 16-bit sRGB through lcms2.
The output SHALL be interleaved RGB at 16 bits per channel.

#### Scenario: A neutral grey develops to sRGB grey [raw_develop_neutral_grey]

- **WHEN** a camera whose `ColorMatrix1` is XYZ-D50-to-linear-ProPhoto at D50 with neutral (1, 1, 1) develops a uniform camera value of 0.18 at the as-shot white balance
- **THEN** every output pixel has three equal channels within 64 counts at sRGB 0.18 encoded (0.4613 of full scale) within 0.01

#### Scenario: A warmer temperature makes the image redder [raw_develop_white_balance]

- **WHEN** the same grey develops at 3000 K and at 9000 K, tint 0
- **THEN** the red-to-blue ratio at 9000 K is greater than at 3000 K

#### Scenario: Baseline exposure scales linear light [raw_develop_baseline_exposure]

- **WHEN** a uniform 0.1 grey develops with `BaselineExposure` 0 and with 1
- **THEN** the second result encodes linear 0.2 within 0.01, and a 0.8 grey with `BaselineExposure` 1 clips to white

#### Scenario: The default crop is applied [raw_develop_default_crop]

- **WHEN** a 12 × 10 camera image with a crop at (2, 1) of 8 × 6 develops
- **THEN** the output is 8 × 6 and its top-left pixel is the camera pixel at (2, 1)

#### Scenario: Orientation rotates and mirrors [raw_develop_orientation]

- **WHEN** a 4 × 2 image with one marked corner develops with each orientation 1–8
- **THEN** orientations 5–8 produce 2 × 4 and 1–4 produce 4 × 2, and the marked pixel lands where the TIFF orientation table puts it

### Requirement: The preview copy is a box-averaged camera image

`downscale_camera` SHALL return camera RGB whose long side is at most the
requested size, keeping the aspect ratio (each side at least 1), with each
output pixel the mean of the source pixels it covers. An image already within
the size SHALL be returned unchanged.

#### Scenario: Downscaling averages and bounds the size [raw_preview_downscale]

- **WHEN** a 40 × 20 image of 2 × 2 checker blocks of 0 and 4000 is downscaled to 10, and a 6 × 4 image to 10
- **THEN** the first is 10 × 5 with every sample 2000 within 1, and the second is returned unchanged

### Requirement: Embedded Camera Raw settings are readable

`raw_xmp` SHALL return the bytes of a TIFF-based raw file's IFD0 XMP tag (700),
or nothing when the tag is absent or the bytes are not TIFF.
`crs_settings_from_xmp` SHALL read the `crs:` Basic controls from an XMP packet.

#### Scenario: A DNG's embedded crs settings are read [raw_xmp_embedded]

- **WHEN** a synthetic DNG carries an XMP packet with `crs:Temperature="4300"` and `crs:Exposure2012="+0.50"`
- **THEN** `raw_xmp` returns the packet and the settings read 4300 and 0.5

#### Scenario: A DNG without XMP has none [raw_xmp_absent]

- **WHEN** a synthetic DNG without tag 700, and PNG bytes, are read
- **THEN** `raw_xmp` returns nothing for each
