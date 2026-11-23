## MODIFIED Requirements

### Requirement: An RW2 is normalized to its sensor borders with Panasonic tags

The active area SHALL run from `SensorTopBorder` and `SensorLeftBorder` up to,
not including, `SensorBottomBorder` and `SensorRightBorder`; borders outside
the plane or enclosing nothing SHALL be refused as invalid. `CFAPattern`
(`0x0009`: 1 RGGB, 2 GRBG, 3 GBRG, 4 BGGR) counts from the sensor's corner and
SHALL be reported from the active area's corner. The black SHALL be
`0x001C`-`0x001E` for red, green and blue, 0 when absent, plus 15 for
RawFormat 3 and 4; the white SHALL be the green linearity limit `0x000F` when
it lies above the blacks and within the coding's range, else that range's top
(4095 for words and RawFormat 3 and 4, the bit depth's top otherwise), and
for RawFormat 2 at most 3967, where its data saturates (observed on the FZ8
and FZ18). Word files SHALL scale both by 16. The as-shot neutral SHALL be the green level
over each of the red, green and blue white-balance levels `0x0024`-`0x0026`,
and absent when one is zero or missing. The crop `0x002F`-`0x0032` (top,
left, bottom, right in sensor coordinates) SHALL become the default crop when
it lies in the active area, else the whole active area is the crop. Make,
model and orientation SHALL come from IFD0. Colour matrices SHALL come only
from the fitted camera table. A malformed file SHALL fail without a panic and
without an allocation beyond the image budget.

#### Scenario: Odd borders shift the colour pattern [rw2_borders]

- **WHEN** each CFA code is read under borders with even and odd top and left, with a different black per colour
- **THEN** the plane is the area inside the borders, each colour normalized by its own black, and the pattern is that of the area's first four photosites

#### Scenario: The crop counts from the sensor [rw2_crop]

- **WHEN** a file with borders 3, 5 records a crop inside them, one reaching outside them, and none
- **THEN** the first gives that crop less the borders and the other two the whole active area

#### Scenario: Camera tags and white balance [rw2_tags]

- **WHEN** a file records make, model, orientation 6 and levels 512, 256, 384, and another a zero red level
- **THEN** they are reported, the neutral is 0.5, 1, 2/3 and absent for the zero, and the image demosaics

#### Scenario: A body without colour is refused at open [rw2_no_colour]

- **WHEN** a decodable DMC-FZ1 file, a body the matrix table does not hold, goes through `decode_raw_source`
- **THEN** it is refused as unsupported, saying the camera colour for Panasonic DMC-FZ1 is not supported yet

#### Scenario: RawFormat 2 saturates at 3967 [rw2_saturation]

- **WHEN** a RawFormat 2 file and a 12-bit word file, each with white tag 4095, hold 3967 at every photosite
- **THEN** the RawFormat 2 plane is 65535 throughout and the word plane is 3967 / 4095 of 65535

#### Scenario: Hostile files fail cleanly [rw2_hostile]

- **WHEN** files of each plain coding are cut at every length, have header and directory bytes corrupted, declare reversed or outside borders, or declare a width over the budget
- **THEN** every cut, bad border and oversize file is an error and none panics

#### Scenario: Real camera files [rw2_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` holds RW2, RWL or Panasonic RAW files
- **THEN** each one of a known coding reads, is not flat and has green phases that agree, and each other one is refused naming its RawFormat
