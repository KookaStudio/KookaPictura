# Test plan

This change has no spec scenarios (`skip_specs: true`). The rows below map the
issue's acceptance criteria and the design risks to runnable checks.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Normal marquee follows the drag | Call `marquee_rect` with `MarqueeStyle::Normal`, no modifiers, no mirror | Integer rect is the normalized drag box | `pictura-select::marquee_rect_normal` |
| Fractional coordinates round half away from zero | Call Normal with fractional endpoints and a `.5` anchor | Exact qRound rect; a truncation regression fails | `pictura-select::marquee_rect_rounds_half_away_from_zero` |
| Shift squares the drag | Call with `shift = true` | Width and height equal the longer drag axis | `pictura-select::marquee_rect_shift_squares` |
| Mirror centres on the press point | Call with `mirror = true` | Rect spans twice the delta around the anchor | `pictura-select::marquee_rect_mirror` |
| Fixed Ratio portrait | Call with a 1:2 ratio and a drag taller than wide | Height wins; width = height × ratio | `pictura-select::marquee_rect_fixed_ratio_portrait` |
| Fixed Ratio landscape | Call with a 2:1 ratio and a drag wider than tall | Width wins; height = width / ratio | `pictura-select::marquee_rect_fixed_ratio_landscape` |
| Fixed Ratio keeps the C++ half-pixel rounding | Call with a 5.5:1 ratio and a 7.5 px drag | Width and anchor match the deleted C++ rect | `pictura-select::marquee_rect_fixed_ratio_half_pixel` |
| A missing ratio falls back to the unconstrained drag | Call Fixed Ratio with ratio `(0,0)` | Rect matches Normal | `pictura-select::marquee_rect_fixed_ratio_without_a_ratio` |
| Fixed Size centres on the press point | Call with an odd fixed size | Rect is exactly the size, centred with the C++ integer truncation | `pictura-select::marquee_rect_fixed_size_centres_odd_size` |
| Close radius includes the boundary | Call `within_radius` inside, exactly on, and outside the radius | True, true, false | `pictura-select::within_radius_includes_the_boundary` |
| A negative radius rejects every point | Call `within_radius` with `-1.0` | False | `pictura-select::within_radius_rejects_a_negative_radius` |
| An unknown style value is rejected | Call `MarqueeStyle::from_i32(9)` | `None` | `pictura-select::marquee_style_from_i32` |
| Polygonal close radius uses document distance | Click 4 px then 7 px from the first vertex | First closes and commits; second stays open | `tst_selection_preview::polygonalCloseRadiusUsesDocumentDistance` |
| Magnetic close radius scales with zoom | At zoom 2, click 3 px then 4 px from the first vertex | First closes and commits; second stays open | `tst_selection_preview::magneticCloseRadiusScalesWithZoom` |
| The Qt marquee preview still closes on the border | Run the existing marquee preview test after the adapter swap | Preview contours unchanged | `tst_selection_preview::marqueePastTheEdgeClosesAlongTheBorder` |
| The C++ self-test still sees the same rects | Run the headless self-test | `tsc_marquee_geometry` passes with the same rects | manual: `./build/pictura --headless --self-test` |
| The clip is unchanged | Run the existing clip tests after the change | Border close, degenerate trace, and self-overlap behave as before | `tst_selection_preview::clipToCanvasClosesAlongTheBorder`, `tst_selection_preview::clipToCanvasKeepsATraceThatEnclosesNothing`, `tst_selection_preview::clipToCanvasClosesASelfOverlappingShape` |
| Workspace gates | Run fmt, clippy, nextest, ctest, openspec validate, and verify-fast | All pass | manual: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo nextest run --workspace`; `ctest --test-dir build -R '^tst_'`; `openspec validate --all --strict`; `bash scripts/verify-fast.sh` |

> Superseded by `selection-marquee-feedback`: `marquee_rect_fixed_size_centres_odd_size`
> is now `marquee_rect_fixed_size_follows_the_pointer`, and
> `polygonalCloseRadiusUsesDocumentDistance` is now
> `polygonalCloseRadiusScalesWithZoom`.
