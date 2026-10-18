# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A spin blurs inside its ellipse only | Spin a striped picture with one centred spin | Pixels inside the feather ring change; pixels outside the ellipse are bit-identical | `pictura-filters::spin_blurs_inside_its_ellipse_only` |
| The smear shortens past the feather ring | Pull the feather ring in to 0.2 and sample the arc length along a ray | It never grows past the ring and is under 0.6× the ring's halfway out, 0.1× near the outline | `pictura-filters::the_smear_shortens_past_the_feather_ring` |
| The blur follows circles round the rotation centre | Spin a picture of concentric rings and one of radial spokes about the same centre | The rings stay within rounding; the spokes change | `pictura-filters::spin_follows_circles_round_the_pivot` |
| A moved rotation centre moves the blur | Apply one spin with a zero and a non-zero pivot offset | The outputs differ | `pictura-filters::moved_pivot_moves_the_blur` |
| Strobe flashes separate the trail | Spin a lone bright pixel continuously and with Strobe Strength 100, four flashes | The strobe arc has more dark gaps between lit taps than the continuous arc | `pictura-filters::strobe_separates_the_arc` |
| Invalid spins are rejected | Apply spins with NaN, Blur Angle 361, radius 0, flashes 0, duration 400, 17 spins | Each returns `InvalidParams`; the buffer is unchanged | `pictura-filters::invalid_spins_are_rejected` |
| Spins alone match their own kernel | Run `blur_gallery` with only spins and `spin_blur` with the same strobe and noise | Bit-identical buffers | `pictura-filters::spins_alone_match_their_kernel` |
| Spins add to a radius effect | Run a Field pin, a spin, and both combined | The combined output differs from each alone | `pictura-filters::spins_add_to_a_radius_effect` |
| The workspace opens on the default spin | Open the Spin Blur workspace from the menu entry's spec | Spin mode, one centred circle, row checked, Blur Angle 15, Motion Effects current with Duration 1 shown, Effects tab disabled, menu entry enabled | `tst_spin_blur::workspaceOpensOnTheDefaultSpin` |
| Spins are edited on the canvas | Drag a radius handle and a feather dot; Alt-drag, then drag the marker; drag the ring; click empty picture; press Delete | Each edit lands in `values()` as the scenario states | `tst_spin_blur::canvasEditsSpins` |
| The duration shows only for Spin Blur | Open in Spin Blur, then select Path Blur | Duration shown, then hidden; strength and flashes shown both times | `tst_spin_blur::durationShowsOnlyForSpinBlur` |
| The preview spins the picture | Toggle Preview off/on after raising the Blur Angle to 180 | The preview changes at the centre and keeps the corners | `tst_spin_blur::previewSpinsThePicture` |
| Spin Blur commits once | Dispatch the menu command and accept with Blur Angle 40 | 26 slots, one `Filter: Spin Blur` state, Last Filter `spin-blur` with those slots | `tst_spin_blur::menuCommitsOnce` |
| A spin list round-trips | `setValues` two spins and non-default strobe; map the list in Rust | `values()` returns it; the mapping reads two spins and that strobe, and rejects a short list | `tst_spin_blur::spinListRoundTrips`, `pictura-app::spin_blur_defaults_and_accepts_spins` |
| Field and Spin commit together | Open Field Blur, switch Spin Blur on, read `values()`, reopen as `blur-gallery`; map a combined list in Rust | The layout is field, empty iris/tilt/path, the spin, motion, duration, tail; it round-trips; the mapping shares the strobe | `tst_spin_blur::combinesWithFieldBlur`, `pictura-app::blur_gallery_reads_the_spin_section` |
