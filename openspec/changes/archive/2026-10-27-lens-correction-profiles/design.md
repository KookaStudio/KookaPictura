# Design: lens-correction-profiles

Decisions taken with the project owner on #932:

1. Real profiles come from lensfun's XML database, **read at runtime** from
   where the system or the user installed it. Nothing from lensfun is copied
   into the tree.
2. Search Online and Save Online Profile Locally stay **disabled**.
3. Exactly **one sub-profile** is picked: the closest by focal length, then
   f-stop, then focus distance. There is no interpolation.
4. Prefer RAW Profiles works from an optional `raw` flag in the in-tree
   format. lensfun has no such flag.
5. Saved settings carry the **resolved coefficients** rather than a profile
   index.
6. In-tree profiles are also loaded from a user folder.

## Engine lens model

`LensModel` stays the one type the map reads. Its radius `r` is still over
the layer's half-diagonal.

| Field | Before | After |
|---|---|---|
| `distortion` | `[k1, k2, k3]` on `r², r⁴, r⁶` | `[c1 … c6]`: source radius `r · (1 + c1 r + c2 r² + … + c6 r⁶)` |
| `ca` (was `ca_scale`) | one scale per channel | per channel `[v, c, b]`: scale `v + c r + b r²` |
| `vignette` | `[v1, v2, v3]` on `r², r⁴, r⁶` | unchanged |

`NONE` is all-zero distortion, `ca = [[1,0,0]; 3]`, and zero vignette. The
in-tree `distortion = k1 k2 k3` fills `c2, c4, c6`, and `ca = r g b` fills
each channel's `v`. Existing profiles therefore map exactly as before. Each
fringe slider still multiplies its channel's whole scale.

`LensModel::rescaled(t)` gives the model for an image whose half-diagonal is
`t` times the calibration's: term `rⁱ` is multiplied by `tⁱ`, and the
vignette's `r²ⁱ` term by `t²ⁱ`. With `t = crop_cal / crop_image` this is
lensfun's crop-factor rule. When either crop factor is unknown, `t = 1`.

## lensfun → engine

lensfun's conventions come from `libs/lensfun/mod-coord.cpp`,
`mod-subpix.cpp`, and `mod-color.cpp`; its manual gives the formulas.
Distortion and TCA use a radius of 1 at half the calibration sensor's
**shorter** side. Vignetting (`pa`) uses a radius of 1 at its **corner**.
The calibration's crop factor and aspect ratio (`<cropfactor>`,
`<aspect-ratio>`, default 1.5) apply. A `<calibration>` element may override
both.

For an image on the calibration camera, the engine's half-diagonal radius
`r` is `R / s`, where `s = hypot(aspect, 1)` for distortion and TCA, and
`s = 1` for vignetting. At parse time:

- `poly3` `Rd = Ru (1 − k1 + k1 Ru²)`, `d = 1 − k1`: `c2 = k1 s² / d³`.
- `poly5` `Rd = Ru (1 + k1 Ru² + k2 Ru⁴)`: `c2 = k1 s²`, `c4 = k2 s⁴`.
- `ptlens` `Rd = Ru (a Ru³ + b Ru² + c Ru + d)`, `d = 1 − a − b − c`:
  `c1 = c s / d²`, `c2 = b s² / d³`, `c3 = a s³ / d⁴`.

  As lensfun does, the centre's magnification `d` is divided out. This is
  lensfun's own rescale (`rescale_polynomial_coefficients`) followed by the
  change of unit.
- TCA `linear`: `v = kr` and `kb`, green `1`. TCA `poly3`
  `Rd = Ru (b Ru² + c Ru + v)`: `[v, c s, b s²]` for red and blue.
- Vignetting `pa`: `k1, k2, k3` as is (`s = 1`).
- `acm`, `none`, and any unknown model are skipped. A lens whose `<type>`
  is not `rectilinear` is skipped, because the map is rectilinear. A lens
  with a bad number is skipped; malformed XML fails only its own file.

The conversion is exact algebra on lensfun's formulas. Matching lensfun's
pixels is not claimed: there is no lensfun oracle in CI, and the engine's
resampler is its own.

### Sub-profiles from a lensfun lens

lensfun keys distortion and TCA by focal length, and vignetting by focal
length, aperture, and distance. A lens yields one profile per calibration
crop factor. That profile has one sub-profile per vignetting entry
`(f, a, d)`, plus one per focal length that has distortion or TCA but no
vignetting entry `(f, –, –)`. A sub-profile takes the distortion and TCA
entries at the nearest calibrated focal length, and the vignette only from
its own entry.

ponytail: lensfun interpolates between calibrations; one sub-profile at the
nearest focal length is CS6's model, and between two calibrated focal
lengths the error is the gap between them.

### Picking the sub-profile

The pick compares candidates lexicographically:

1. Focal length, by `|Δ mm|` (ties within 0.5 mm).
2. f-stop, by `|log₂ ratio|` (ties within 1/6 stop).
3. Focus distance, by `|ln ratio|`.

When EXIF gives no value for a criterion, that criterion is skipped. A
sub-profile that lacks a value the shot has ranks after the ones that carry
it. The first sub-profile wins a full tie. Adobe's rule is closed; this is
the approximation recorded in the doc.

## Matching and order

A profile with an empty `model` (every lensfun profile) suits any camera.
It matches when its lens equals the criterion's lens, after folding case,
spaces, and dashes. With no lens criterion, it matches when its maker equals
the camera make. A camera-specific profile (in-tree) keeps the old rule:
the make is required, and a lens match outranks a model match.

The order is by match score, then raw profiles first with Prefer RAW, then
by the crop-factor distance to the criteria camera (unknown last), then by
name.

## Profile identity and the slots

A profile's id is a stable 32-bit FNV-1a hash of `name, make, model, lens`,
folded into `2³⁰ … 2³¹ − 1`. It never collides with the legacy id `1`.
When two installed profiles share an id, the later source wins, in the
order built-in, lensfun, then user.

The kind's slots grow from 16 to 36. The first 16 are unchanged, and the
new ones follow:

| Slot | Meaning | Key | Type |
|---|---|---|---|
| 16 | Sub-profile index | `SubP` | Long |
| 17 | Coefficients present | `PCoe` | Bool |
| 18–23 | Distortion `c1…c6` | `PDs1`…`PDs6` | Double |
| 24–26 | Red `v c b` | `PCR0`…`PCR2` | Double |
| 27–29 | Green `v c b` | `PCG0`…`PCG2` | Double |
| 30–32 | Blue `v c b` | `PCB0`…`PCB2` | Double |
| 33–35 | Vignette `k1 k2 k3` | `PVg1`…`PVg3` | Double |

`from_slots` reads the profile as follows:

- With slot 17 on, the model is built from slots 18–35, gated by the Auto
  checkboxes, whatever the id.
- With slot 17 off, an id of `0` means no profile.
- With slot 17 off, an id of `1` is the legacy index of the bundled
  synthetic profile (a pre-#932 record).
- Any other id is refused.

A descriptor written before #932 has no tail keys and decodes to the tail
defaults (the flag off), so it keeps rendering the bundled profile.

The dialog resolves the coefficients in Rust, in `lc_profile_slots(id, sub,
make, model)`, using the criteria camera's crop factor. It keeps the 20
tail values with the Auto panel. Its `values()` are therefore the head (6),
then Custom (10), then the tail (20).

When the dialog is restored from slots, the stored coefficients are kept
verbatim; it never recomputes them. If the id is not installed, the list
shows "Profile unavailable" and the checkboxes stay enabled. OK then
commits the same correction.

## Profile store (app)

`filter_tools/lens_profiles.rs` loads profiles once per process (a
`OnceLock`). It loads the bundled set, then lensfun, then the user's
in-tree files.

- **lensfun roots, from lowest to highest priority:**
  1. `/usr/share/lensfun`
  2. `/usr/local/share/lensfun`
  3. each `$XDG_DATA_DIRS` entry plus `/lensfun`
  4. `/var/lib/lensfun-updates`
  5. `$XDG_DATA_HOME/lensfun` (default `~/.local/share/lensfun`) and its
     `updates/`

  Within each root, `version_1/`, then `version_2/`, then the root itself
  are read. A file with the same name in a later directory replaces an
  earlier one, as lensfun does. `PICTURA_LENSFUN_DIR`, when set, is the
  only root.
- **User profiles:** `$XDG_DATA_HOME/pictura/lens-profiles/*.txt`, which is
  Qt's `AppDataLocation` for this binary.

Directory discovery is a pure function of the environment, so it can be
tested without the `OnceLock`.

## In-tree format additions

```text
[profile]
name = ACME 24-70mm
make = ACME Cameras
model = ACME One
lens = ACME 24-70mm f/2.8
raw = yes                # optional; Prefer RAW ranks it first
distortion = -0.05 0 0   # defaults every [sub] starts from
[sub]
focal = 24
aperture = 2.8
distance = 3
distortion = -0.12 0.02 0
```

A profile with no `[sub]` has one sub-profile built from its own keys. A
`[sub]` outside a profile is an error, and so is a `[sub]` key such as
`focal` at the profile level.

## Dialog

- **Search Criteria** gets a `QToolButton` pop-up (`lcSearchMenu`) with a
  checkable **Prefer RAW Profiles** (`lcPreferRaw`). The choice lasts for
  the session (a file-scope static) and refills the list.
- **Lens Profiles** shows a context menu on right-click. It lists the
  sub-profiles ("24 mm, f/2.8, 3 m"; "24 mm"; "Default"), checks the
  current one, and ends with a disabled **Save Online Profile Locally**.
  Picking a sub-profile recomputes the tail and pushes the preview.
  `subProfileMenu()` builds the menu for tests.
- **Search Online** stays disabled, and its tooltip names the reason.
- The camera's crop factor changes when the make or model changes, so the
  tail is recomputed whenever the list is refilled.

## Out of scope

- Set Lens Default, and Save / Load Settings (#933).
- Interpolating between calibrations.
- lensfun's `acm` models.
- Non-rectilinear lenses.
- Lens decentering (`<center>`).
- Camera-to-mount filtering.
