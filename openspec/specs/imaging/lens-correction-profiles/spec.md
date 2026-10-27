# imaging/lens-correction-profiles Specification

## Purpose
Real lens profiles for the Lens Correction filter: profiles read at runtime
from the system or user lensfun database and from in-tree profile files,
sub-profiles picked from the photo's EXIF, Prefer RAW Profiles ordering, and
the resolved coefficients carried in the filter's saved settings.

## Requirements

### Requirement: Lens profiles from the lensfun database

The system SHALL read lens profiles at runtime from the lensfun XML database
installed on the system or by the user, and SHALL NOT bundle any lensfun
data. Each lensfun lens SHALL yield one profile per calibration crop factor,
with sub-profiles from its calibration entries. Distortion `poly3`, `poly5`,
and `ptlens`, TCA `linear` and `poly3`, and vignetting `pa` SHALL be
converted exactly to the engine's half-diagonal radius, following lensfun's
coordinate conventions (half the calibration's shorter side for distortion
and TCA, its corner for vignetting) and dividing out a `ptlens` or `poly3`
centre magnification as lensfun does. `acm` and unknown models SHALL be
skipped, a non-rectilinear lens SHALL be skipped, a lens with a malformed
number SHALL be skipped, and malformed XML SHALL fail only its own file.
Applied to an image, a profile SHALL be rescaled by the calibration's crop
factor over the camera's when both are known.

#### Scenario: lensfun XML yields profiles and sub-profiles [lcp_lensfun_parse]

- **WHEN** `lens_correction::tests::lensfun_xml_yields_profiles_and_subprofiles`
  parses a synthetic database with a zoom lens calibrated at two focal
  lengths and two vignetting apertures, and a second calibration crop factor
- **THEN** the lens gives one profile per crop factor, one sub-profile per
  vignetting entry plus one per focal length without one, each with the
  nearest focal length's distortion and TCA, and lens-only profiles carry
  an empty model

#### Scenario: Unsupported entries are skipped [lcp_lensfun_skip]

- **WHEN** `lens_correction::tests::lensfun_skips_what_it_cannot_model`
  parses `acm` entries, a fisheye lens, a lens with a bad number, and
  malformed XML
- **THEN** the `acm` entries and the two lenses are skipped, the rest of the
  file loads, and the malformed file is an error

#### Scenario: Coefficients convert to the half-diagonal [lcp_lensfun_convert]

- **WHEN** `lens_correction::tests::lensfun_coefficients_convert_to_the_half_diagonal`
  maps points through converted `poly3`, `poly5`, `ptlens`, TCA `poly3`, and
  `pa` entries on a 3:2 calibration
- **THEN** each mapped radius and gain equals lensfun's formula evaluated in
  lensfun's own units, with the `ptlens` centre magnification divided out

#### Scenario: A profile rescales to the camera's crop factor [lcp_crop_rescale]

- **WHEN** `lens_correction::tests::profile_rescales_to_the_camera_crop`
  resolves one sub-profile for a camera of the calibration's crop factor, a
  camera of a larger one, and an unknown camera
- **THEN** the same crop and an unknown camera keep the coefficients, and
  the larger crop maps a radius `r` as the calibration maps `r · crop_cal /
  crop_camera`

### Requirement: Lens profile search paths and user profiles

lensfun files SHALL be found under `/usr/share/lensfun`,
`/usr/local/share/lensfun`, each `$XDG_DATA_DIRS` entry, `/var/lib/lensfun-updates`,
and `$XDG_DATA_HOME/lensfun` (with its `updates/`), in that priority order,
each in its `version_1/`, `version_2/`, or the root itself; a file name found
again later SHALL replace the earlier file. `PICTURA_LENSFUN_DIR`, when set,
SHALL be the only lensfun root. In-tree profiles SHALL also load from
`$XDG_DATA_HOME/pictura/lens-profiles/*.txt` (default
`~/.local/share/pictura/lens-profiles`). With no lensfun installed and no
user profiles, only the bundled synthetic profile SHALL be offered.

#### Scenario: Search directories follow XDG and the override [lcp_search_dirs]

- **WHEN** `lens_profiles::tests::lensfun_dirs_follow_xdg_and_the_override`
  computes the roots for a given environment, with and without
  `PICTURA_LENSFUN_DIR`
- **THEN** the system, `XDG_DATA_DIRS`, update, and user roots come in order,
  and the override is the only root

#### Scenario: A later file replaces an earlier one [lcp_search_replace]

- **WHEN** `lens_profiles::tests::later_files_replace_earlier_ones` loads two
  roots holding a file of the same name with different lenses
- **THEN** only the later root's lenses load, and a file only the earlier
  root has still loads

#### Scenario: User profiles load from the data home [lcp_user_profiles]

- **WHEN** `lens_profiles::tests::user_profiles_load_from_the_data_home` and
  `tst_lens_profiles::userAndLensfunProfilesAreListed` load a user folder
  with one in-tree profile and a lensfun root with one lens
- **THEN** the bundled, lensfun, and user profiles are all installed, and a
  file that fails to parse is skipped

### Requirement: Lens sub-profiles

Every profile SHALL hold one or more sub-profiles, each with an optional
focal length, f-stop, and focus distance. The in-tree format SHALL take
`[sub]` blocks inside a profile, each starting from the profile's own
coefficients. The dialog SHALL pick one sub-profile, the closest to the
photo's EXIF by focal length, then f-stop (in stops), then focus distance;
a criterion EXIF does not give SHALL be skipped. Right-clicking the Lens
Profiles list SHALL list the selected profile's sub-profiles with the
current one checked; picking one SHALL apply it.

#### Scenario: The format reads sub-profiles and the raw flag [lcp_sub_format]

- **WHEN** `lens_correction::tests::profile_parse_reads_subprofiles_and_raw`
  parses a profile with defaults and two `[sub]` blocks, one with no blocks,
  a `[sub]` before any profile, and `focal` at the profile level
- **THEN** the subs inherit and override the defaults, the plain profile has
  one sub-profile, `raw = yes` sets the flag, and the two misplaced lines are
  errors naming their line

#### Scenario: The closest sub-profile is picked [lcp_sub_pick]

- **WHEN** `lens_correction::tests::subprofile_pick_is_closest_focal_then_aperture_then_distance`
  picks among sub-profiles for shots with and without each EXIF value
- **THEN** the nearest focal length wins first, then the nearest f-stop in
  stops, then the nearest distance; a missing EXIF value is skipped and a
  full tie keeps the first

#### Scenario: EXIF picks the sub-profile in the dialog [lcp_sub_auto]

- **WHEN** `tst_lens_profiles::exifPicksTheClosestSubprofile` opens the
  dialog on the 50 mm f/2.8 `metadata.psd` with a synthetic lensfun lens
  calibrated at 24, 50, and 70 mm
- **THEN** that lens is listed for the camera make, its 50 mm f/2.8
  sub-profile is selected, and the committed slots carry its coefficients

#### Scenario: The sub-profile menu switches sub-profiles [lcp_sub_menu]

- **WHEN** `tst_lens_profiles::subprofileMenuSwitchesTheSubprofile` builds
  the context menu and triggers another sub-profile
- **THEN** the menu lists every sub-profile with the current one checked,
  the slots' sub-profile index and coefficients change, and the preview
  changes

### Requirement: Lens profile matching and Prefer RAW Profiles

A profile with no camera model SHALL suit any camera: it matches when its
lens equals the lens criterion, or, with no lens criterion, when its maker
equals the camera make. The list SHALL be ordered by match, then (with
Prefer RAW Profiles) raw profiles first, then by how near the calibration's
crop factor is to the camera's, then by name. Search Criteria SHALL offer
Prefer RAW Profiles as a checkable pop-up item that reorders the list and
holds for the rest of the session.

#### Scenario: Lens-only profiles match and sort by sensor [lcp_match]

- **WHEN** `lens_correction::tests::lens_profiles_match_any_camera_and_sort_by_sensor`
  searches lens-only and camera-specific profiles for a camera of crop 1.5
- **THEN** a lens-only profile matches by lens for any make or by maker
  without a lens, camera-specific profiles keep the make rule, and among
  equal matches the nearer crop factor comes first

#### Scenario: Prefer RAW ranks raw profiles first [lcp_prefer_raw]

- **WHEN** `lens_correction::tests::prefer_raw_ranks_raw_profiles_first` and
  `tst_lens_profiles::preferRawReordersTheList` search with Prefer RAW off
  and on
- **THEN** off keeps the sensor-size order; on moves the raw profile ahead
  of an equal match, and the dialog's list follows the toggle

### Requirement: Lens profile identity in the slots

Profile ids SHALL be stable hashes of the profile's name, make, model, and
lens, in `2³⁰…2³¹−1`. The `lens-correction` slots SHALL carry the
sub-profile index, a coefficients-present flag, and the resolved coefficients
after the 16 dialog slots. With the flag on, the profile SHALL be built from
the stored coefficients whatever the id; with it off, id 1 SHALL be the
bundled profile (a record written before the change) and another non-zero id
SHALL be refused. A restored profile that is not installed SHALL show
"Profile unavailable" and keep its stored correction.

#### Scenario: Profile ids are stable [lcp_ids]

- **WHEN** `lens_correction::tests::profile_ids_are_stable` hashes profiles
  loaded in two orders
- **THEN** each profile has the same id both times, ids differ between
  profiles, and every id is at least 2³⁰

#### Scenario: The slots carry the coefficients [lcp_slots]

- **WHEN** `lens_correction::tests::slots_carry_the_profile_coefficients`
  maps slots with the flag on and an unknown id, a 16-slot-era record with
  id 1, and the flag off with an unknown id
- **THEN** the first builds the stored model gated by the checkboxes, the
  second the bundled profile, and the third is refused

#### Scenario: An unavailable profile keeps its settings [lcp_unavailable]

- **WHEN** `tst_lens_profiles::unavailableProfileKeepsItsSettings` loads
  slots whose profile id is not installed and accepts
- **THEN** the list reads "Profile unavailable", the Correction checkboxes
  are enabled, and the committed slots equal the loaded ones

### Requirement: Online lens profiles stay disabled

Search Online and Save Online Profile Locally SHALL be shown and disabled,
with a tooltip saying no online profile service is available.

#### Scenario: The online controls are disabled [lcp_online]

- **WHEN** `tst_lens_profiles::onlineProfilesStayDisabled` opens the dialog
  with a profile selected and builds the sub-profile menu
- **THEN** Search Online is disabled with its tooltip, and the menu's Save
  Online Profile Locally is disabled
