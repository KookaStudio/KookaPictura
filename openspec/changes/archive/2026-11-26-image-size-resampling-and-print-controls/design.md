## Context

Image Size already applies resolution and geometry in a single document operation. The render crate resizes byte and native planes in spillable strips, with a separate dense test oracle. Existing Scale Effects code knows which fields are lengths or overlay scales.

## Decisions

- Keep each cubic axis at four taps to preserve the window and memory budget. Keep ordinary Keys bicubic at a = −0.75. Use a = −0.5 for Smoother and a = −1 for Sharper, reducing or increasing the negative lobes respectively. Automatic dispatches by source and destination extents on each axis, with ordinary bicubic for unchanged axes. These are behavioral approximations of closed Adobe kernels.
- Use raw finite f32 sample values, accumulating in f64 and returning f32. Treat nonfinite input as zero for interpolation. Keep byte and u16 rounding and range behavior. Route all byte and native Image Size planes through the same method selection.
- Reuse the style field-scaling logic inside the resize transaction, visiting the cloned document tree and scaling all stored supported layer effects, including disabled effects. Keep Layer > Layer Style > Scale Effects' existing enabled-only behavior. Scale by the destination/source ratio on the longer source axis when the target is proportional; do not scale at identity. Treat a target as proportional when the positive scale-factor intervals for both axes intersect: for source extent `s` and destination extent `d > 1`, the interval is `[(d − 0.5) / s, (d + 0.5) / s)`; for `d = 1`, it is `(0, 1.5 / s)`, accounting for rounding and the one-pixel clamp. Apply this same test to constrained edits and Fit To targets; suppress scaling when the intervals do not intersect. Older recorded actions default to Scale Styles off. Newly recorded choices are explicit.
- Keep the existing Image Size layout. Enable Auto Resolution in Fit To and use the existing resolution edit path. The screen unit is lines/in or lines/cm. Treat Columns as a width unit per Adobe's column-sizing guidance. Use one width and interior gutters for full columns and a proportional width for a fraction smaller than one. Persist width and gutter in inches and expose the same physical unit choices in Units & Rulers. Points and picas honor the existing point-size choice.
- A layerless RGB or Gray resize carries the resized native color and signed merged-alpha samples into its synthesized layer, so a compatible save retains the layer samples. The existing native compositor still clamps the saved merged 32-bit composite to 0–1; changing that compositor is outside this change.
- Keep Preserve Details' existing dispatch and other commands' existing defaults. This change covers Image Size and the preferences it consumes.

## Risks

Exact Adobe kernels and column rounding require CS6 comparison. Fit To sizing and ordinary resize carry their existing command semantics. The tiled test oracle keeps independent taps so a shared dispatch error can be detected.

## References

- Repository Image Size contract in `docs/04-image-ops/image-size.md`.
- [Adobe CS6 Printing Guide](https://helpx.adobe.com/content/dam/help/en/indesign/pdf/CS6_PrintGuide.pdf).
- [Adobe column-sizing guide](https://helpx.adobe.com/in/photoshop/using/specifying-columns-image.html).
