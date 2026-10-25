# Spec Delta

## MODIFIED Requirements

### Requirement: Grayscale, RGB, CMYK, and Lab conversions

`convert_mode(doc, target, profiles)` SHALL convert to Grayscale, RGB, CMYK, or
Lab, keep the document's bit depth, and return an error without mutating `doc`
when the conversion is unavailable. `profiles` MAY be absent for a library
caller; the application SHALL always supply the working RGB and working CMYK
profiles from Color Settings.

- Grayscale SHALL replace each composite and layer RGB triple with its
  Rec. 601 luma `(299 R + 587 G + 114 B + 500) / 1000` and leave a single
  color channel. A retained 16/32-bit store SHALL be reduced at its own depth.
- RGB SHALL replicate a gray plane into three, or drop a recorded source mode
  and its stores.
- CMYK SHALL encode each RGB plane set through the working CMYK profile when one
  is supplied, using the working RGB profile as the source and the supplied
  rendering intent and black-point-compensation setting. When no working CMYK
  profile is supplied, CMYK SHALL fall back to the documented profile-free black
  generation (`K = max(R, G, B)`, each ink `round(255 · C / K)` in the stored
  convention). After a profile-aware conversion the document SHALL carry the
  working CMYK profile in image resource `1039`.
- Lab SHALL encode with the profile-free CIELAB transform.

For CMYK and Lab, the encoded planes SHALL be retained as the document's and
each layer's source store, and the working RGB SHALL become what those planes
decode to. Leaving a Bitmap document SHALL first rebuild a Background layer
from its composite.

#### Scenario: Grayscale is a rounded luma

- **WHEN** an RGB layer pixel `(10, 200, 30)` is converted to Grayscale
- **THEN** the layer has one color channel and the pixel is `124`

#### Scenario: A CMYK conversion uses the working profile

- **WHEN** an RGB document is converted to CMYK with Photocraft Coated CMYK as the working CMYK profile
- **THEN** the retained CMYK planes are the lcms2 conversion from the working RGB profile, image resource `1039` carries the working CMYK profile, and a re-read with the profile yields the converted working RGB within tolerance

#### Scenario: A CMYK conversion without a working profile uses the fallback

- **WHEN** a library caller converts an RGB document to CMYK without supplying a working CMYK profile
- **THEN** the retained planes use the documented profile-free black generation and the result remains readable by the profile-free read path

#### Scenario: A CMYK conversion saves as CMYK

- **WHEN** a converted CMYK document is written and read back
- **THEN** the file is header mode CMYK, its planes equal the retained store,
  and the read working RGB equals the converted working RGB within the
  profile round-trip tolerance
