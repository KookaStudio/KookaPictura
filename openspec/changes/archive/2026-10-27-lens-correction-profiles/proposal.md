# Proposal: lens-correction-profiles

## Why

Issue #932, a follow-up to #347. Lens Correction ships with an in-tree
profile format and one synthetic profile, so real photos show "No matching
profiles found". Each profile has one set of coefficients, so nothing varies
with focal length, f-stop, or focus distance. Search Criteria has no Prefer
RAW Profiles, and the Search Online / Save Online Profile Locally question
is open (`docs/06-filters/lens-correction.md`, `FILT-101`).

## What Changes

- **Real profiles from the system lensfun database, read at runtime.**
  Nothing from lensfun is bundled: its data is CC BY-SA 3.0, which cannot be
  relicensed under GPL-3.0. The app reads whatever lensfun XML the system
  or the user has installed (`lensfun-data`, `lensfun-update-data`,
  `~/.local/share/lensfun`), and `PICTURA_LENSFUN_DIR` replaces the search
  for tests. The lens models are `poly3`, `poly5`, and `ptlens` for
  distortion, `linear` and `poly3` for TCA, and `pa` for vignetting. They
  are converted exactly into the engine's half-diagonal radius, taking
  lensfun's crop-factor and aspect-ratio rules into account. `acm` entries
  and non-rectilinear lenses are skipped.
- **User profiles in the in-tree format** are read from
  `$XDG_DATA_HOME/pictura/lens-profiles/*.txt`. The format gains `raw = yes`
  and `[sub]` blocks (focal, aperture, distance).
- **Sub-profiles.** Every profile holds one or more sub-profiles. The dialog
  picks the single closest one to the photo's EXIF, comparing focal length,
  then f-stop, then focus distance. Right-clicking the profile lists the
  sub-profiles so another can be picked.
- **Search Criteria pop-up** with **Prefer RAW Profiles**, which ranks raw
  profiles first. The list is otherwise sorted by match, then by how close
  the calibration's crop factor is to the camera's.
- **The engine's lens model grows.** Distortion becomes a polynomial in `r`
  with terms `r¹…r⁶`, so `ptlens` and `poly3` are exact. Each channel's
  chromatic-aberration scale becomes `v + c·r + b·r²`.
- **Saved settings carry the resolved coefficients.** The `lens-correction`
  kind grows from 16 to 36 slots: a sub-profile index, a
  "coefficients present" flag, and the 18 resolved coefficients. A document,
  Last Filter, or a smart filter therefore renders the same on any machine,
  whatever lensfun has installed. Profile ids become stable hashes. A
  profile that is not installed shows "Profile unavailable" and keeps its
  settings. Older 16-slot records decode with the flag off and still resolve
  the bundled profile.
- **Search Online and Save Online Profile Locally stay disabled.** Adobe's
  service has no replacement, and the tooltip says so.
- New dependency: `quick-xml` (MIT), to parse the lensfun XML. Its only
  required dependency is `memchr`, which is already in the lock file.

## Capabilities

### New Capabilities

- `imaging/lens-correction-profiles`: the lensfun database, user profiles,
  sub-profiles, Prefer RAW Profiles, the slot encoding of a profile, and
  the disabled online buttons.

### Modified Capabilities

- `imaging/lens-correction`: the profiles requirement (unknown profiles are
  refused only when the slots carry no coefficients) and the commit (36
  slots).
- `imaging/lens-correction-smart-filter`: the descriptor (36 slots), the
  rendering (the profile comes from the stored coefficients), and the Smart
  Object commit (36 slots).

## Impact

- `pictura-filters`: `distort/lens_correction/{mod,map,profile,lensfun}.rs`,
  the catalogue row (36 slots), exports, and test fixtures; `quick-xml`.
- `pictura-codec`: `lens_correction` slot keys and defaults.
- `pictura-render`: the smart-filter tests.
- `pictura-app`: `filter_tools/lens_correction.rs`, a new
  `filter_tools/lens_profiles.rs` (the store and its search), and
  `lens_correction_panels.*` / `lens_correction_dialog.*`; the
  `tst_lens_correction` slot fixtures and a new `tst_lens_profiles` suite.
- Docs: `docs/06-filters/lens-correction.md`,
  `docs/dev/doc-code-parity-gaps.md`, `assets/PROVENANCE.md`, `ROADMAP.md`.
