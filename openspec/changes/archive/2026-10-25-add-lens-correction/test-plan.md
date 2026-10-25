# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lc_identity` | Apply default settings (Midpoint 10, Edge Black, Angle 360; then a profile with every part off) | Every sample unchanged | `pictura-filters::default_params_are_bit_identical` |
| `lc_distortion` | Walk a ray to the corner at Remove Distortion −100…+100 | Centre fixed; radius grows along the ray; positive samples nearer the centre at the corner; −100 magnifies the centre 2.5× and keeps the corner | `pictura-filters::distortion_keeps_centre_and_moves_edges_monotonically` |
| `lc_vertical` | Map rows at Vertical +60, −60, +50 | Centre fixed; the top row shows a narrower source span; −60 mirrors +60; verticals converge upward at +50 | `pictura-filters::vertical_perspective_widens_the_top_and_mirrors`, `pictura-filters::vertical_perspective_converges_verticals` |
| `lc_horizontal` | Map the side columns at Horizontal +60 | The right column shows a taller source span | `pictura-filters::horizontal_perspective_narrows_the_right_side` |
| `lc_perspective_fill` | Map the frame corners at several perspectives; apply V +100, H +70 | Every corner on the layer; every pixel opaque | `pictura-filters::perspective_alone_leaves_no_blank` |
| `lc_angle` | Map a square layer at Angle 90 | Centre fixed; source top-left shows at the top right | `pictura-filters::angle_rotates_clockwise_about_the_centre` |
| `lc_scale` | Map a point at Scale 200 % | Halfway to the centre | `pictura-filters::scale_zooms_about_the_centre` |
| `lc_scale_refused` | Apply Scale 0 | Error | `pictura-filters::zero_scale_is_refused` |
| `lc_straighten` | Measure a leaning line both ways; set Angle to undo it | Lean within (−45°, 45°]; Angle wraps to 0–360; a level row shows the line | `pictura-filters::straighten_levels_a_line`, `pictura-app::straighten_returns_the_levelling_angle` |
| `lc_deterministic` | Apply the same settings twice | Byte-identical | `pictura-filters::deterministic` |
| `lc_fringe` | Apply each fringe slider alone; map a point at Red +100 / Blue −100 | Only that channel changes; red further out, blue nearer | `pictura-filters::fringe_scales_only_its_channel` |
| `lc_vignette` | Amount ±100 at Midpoints 0, 50, 100 on flat grey | Centre unchanged; corners dark or light; Midpoint 0 reaches a mid-radius point Midpoint 100 does not | `pictura-filters::vignette_darkens_and_lightens_corners`, `pictura-filters::geometry_free_settings_keep_every_sample_position`, `tst_lens_correction::vignetteLightensThePreview` |
| `lc_edge_transparent` | Shrink to 50 % with Transparency, with and without alpha | Corner clear / white; centre opaque | `pictura-filters::edge_transparency_clears_or_whitens` |
| `lc_edge_color` | Shrink to 50 % with Black and White | Opaque black / white corners | `pictura-filters::edge_colours_fill_opaque` |
| `lc_edge_extend` | Shrink to 50 % with Edge Extension | Corners repeat the source corners | `pictura-filters::edge_extension_repeats_the_edge` |
| `lc_auto_scale` | Pincushion profile with Auto Scale off, on, on + Angle 30 | Clear corners; fully opaque; clear corners again | `pictura-filters::auto_scale_leaves_no_blank` |
| `lc_preview_match` | Render the preview at 100 % and commit | Alpha equal; opaque colour within 1 | `pictura-filters::preview_render_matches_commit` |
| `lc_profile_parse` | Parse the bundled set and malformed text | Bundled parses; errors name the line or profile | `pictura-filters::profile_parse_reports_the_line`, `pictura-filters::builtin_profiles_parse` |
| `lc_profile_match` | Search three profiles by make, model, lens | Make required; lens outranks model; no make, nothing | `pictura-filters::profiles_match_make_then_model_and_lens` |
| `lc_profile_compose` | Vignette-only profile with and without a Custom darkening; clear each checkbox | Corners brighten, then darken back; cleared parts are zero | `pictura-filters::profile_composes_with_custom`, `pictura-app::checkboxes_gate_the_profile_parts` |
| `lc_slots` | Map default, out-of-range, NaN, short, and unknown-profile slot lists | Identity; clamped; refused; refused; refused | `pictura-app::default_slots_are_the_identity`, `pictura-app::slots_map_and_clamp`, `pictura-app::unknown_profile_id_is_refused` |
| `lc_menu_gate` | New RGB 8-bit, 16-bit, 32-bit, Grayscale, CMYK, Lab documents | Enabled for 8/16-bit RGB and Grayscale only; AWA and VP stay 8-bit | `tst_lens_correction::menuFollowsDepthAndMode`, `pictura-app::commits_are_refused_off_eight_bit_rgb_and_gray` |
| `lc_shortcut` | Read the menu action | `Shift+Ctrl+R` | `tst_lens_correction::menuRowHasItsShortcut` |
| `lc_last_gate` | Commit; 16-bit Last Filter; 32-bit Last Filter Settings and Last Filter | 16-bit applies; 32-bit no dialog, no state, refusal names "8- or 16-bit" | `tst_lens_correction::lastFilterRefusesOffSixteenBit` |
| `lc_controls` | Open on an image without EXIF | CS6 title, tabs, Edge items, ranges, defaults; "No matching profiles found"; Search Online disabled; `--` status lines | `tst_lens_correction::opensWithControls` |
| `lc_exif` | Open on `metadata.psd`; toggle Vignette; clear the make | Criteria and status filled ("50mm, f/2.8"); profile chosen; preview changes; no profile | `tst_lens_correction::exifPrefillsSearchAndStatus`, `pictura-app::camera_settings_text` |
| `lc_settings` | Edit sliders, then Default Correction | Custom after the edit; distortion reset, perspective kept | `tst_lens_correction::settingsComboSwitchesToCustom` |
| `lc_tool_distort` | Drag toward the centre, then away | Positive, then negative | `tst_lens_correction::removeDistortionToolDragsSlider` |
| `lc_tool_straighten` | Drag a line leaning clockwise | Angle = 360° − lean; Settings Custom | `tst_lens_correction::straightenToolSetsAngle` |
| `lc_angle_dial` | Set the field to 30; turn the dial to 0 | Dial at 60 (math angle); field 90 | `tst_lens_correction::angleDialAndFieldSync` |
| `lc_tool_grid` | Drag 10, 5 with Move Grid; then 70 across | Offset 10, 5; wraps to 16, 5 | `tst_lens_correction::moveGridToolOffsetsGrid` |
| `lc_grid` | Show Grid at size 32 | 17 lines over 300×200; swatch matches the grid colour | `tst_lens_correction::showGridSizeAndColor` |
| `lc_tool_keys` | Press A, M, H, Z, D | Each tool selected on dialog and canvas | `tst_lens_correction::toolShortcuts` |
| `lc_hint` | Select each tool | Help line reads its hint | `tst_lens_correction::hintFollowsTool` |
| `lc_preview` | Scale 50 %, Preview off and on, then Black Color | Clear; layer pixel; clear; black | `tst_lens_correction::previewToggleShowsSource` |
| `lc_gray` | Open on a Grayscale document | Fringe sliders and Chromatic Aberration disabled; slot 0 | `tst_lens_correction::grayscaleDisablesChromaticAberration` |
| `lc_commit` | Cancel, then OK with Remove Distortion 40 | No change; then one state, 16 slots, pixels changed | `tst_lens_correction::cancelAndOk` |
| `lc_previous` | Commit distortion, fringe, perspective; pick Previous Conversion | Enabled; distortion and fringe restored; perspective 0 | `tst_lens_correction::previousConversionRestoresLastCommit` |
| `lc_last_settings` | Commit, then Last Filter Settings | Committed values shown; Cancel adds no state | `tst_lens_correction::lastSettingsRestoresValues` |
| `lc_catalog` | Load both catalogues | >3,000 bodies, >1,500 lenses; full-frame crop 1; a compact's fixed lens; a lens's flags and range | `pictura-filters::catalogues_load` |
| `lc_lens_names` | Parse prime, zoom, variable-aperture, and rangeless names | Range and aperture, or none | `pictura-filters::lens_names_give_their_range_and_aperture` |
| `lc_exif_names` | Look up "NIKON CORPORATION" / "NIKON D850" and unknown names | "Nikon" / "D850"; unknown pass through | `pictura-filters::exif_names_find_the_catalogue_spelling`, `pictura-app::exif_names_take_the_catalogue_spelling` |
| `lc_lens_settings` | List lenses for Canon with and without 100 mm; open on a 50 mm photo | Own make first, nearest crop first; only covering lenses | `pictura-filters::lenses_follow_the_camera_and_its_settings`, `tst_lens_correction::exifPrefillsSearchAndStatus` |
| `lc_criteria` | Pick Canon, EOS 5D Mark III, EF 24-70mm f/2.8L II | Lists fill; line gives crop, sensor, range, aperture, calibrated corrections; no profile | `tst_lens_correction::criteriaListCamerasAndLenses`, `pictura-app::criteria_info_describes_camera_and_lens` |
| Visual match to CS6 | Correct the same barrel, pincushion, tilted, and fringed photos in CS6 and Pictura at matching slider values | Comparable direction and strength of each correction | manual: no CS6 oracle; the distortion, fringe, vignette, and tilt scales are approximations of closed algorithms |
| Interactive feel | Drag Remove Distortion and the sliders on a 24 MP photo | Preview stays responsive | manual: responsiveness on real hardware is not observable offscreen |
| Grid colour picker | Click the Color swatch, pick a colour | The grid redraws in it | manual: the Color Picker is a modal dialog the suite does not drive |
