# Design: Blend Text Colors Using Gamma and missing fonts

## The curve

CS6 documents the gamma blend only qualitatively: type is blended "using a
gamma" of 1.45 instead of the document's, which changes anti-aliased edges and
the effective opacity of translucent type. No CS6 reference renders exist, so
this is an approximation (rule 5).

Model: dark type over a light backdrop, blended in a γ space instead of the
document's ≈2.2. Solving that blend for the alpha that gives the same result in
the document space gives

    a' = 1 − (1 − a)^(γ / 2.2)

which is monotonic, keeps 0 at 0 and 1 at 1, is the identity at γ = 2.2, and
for γ = 1.45 thins edges and lightens translucent type (a = 0.5 → 0.37). It is
colour-independent, as the issue decides ("a LUT on the text coverage"). So
light type on a dark backdrop thins too, where a true gamma-space blend would
thicken it (`ponytail:` in `text_gamma.rs`).

`a` is the type layer's coverage × opacity × Fill, which is what makes the
setting change translucent type's effective opacity. The layer mask and
Blend If multiply after the curve. "Type layer" means a layer with a decoded
`type_tool`, whether it composites its stored pixels or re-renders its text.

## Where the setting lives

The setting is application-wide, like the incoming-profile policy. The
compositor takes `&Document` everywhere, and the setting must not ride
history snapshots, so it is a process-wide value in `pictura-render`
(`set_text_gamma(Option<f32>)`, default `Some(1.45)`), set by the app at
startup and from Color Settings. Engine tests that need the gamma off live in
their own integration-test binary, so they cannot race a test that composites
type with the default.

## Consequences

- **Saving and exporting** read the CPU composite, so the merged image carries
  the curve.
- **Rasterize Type** funnels through `materialize_text_rgba`, which applies the
  curve to the alpha it stores. Opacity stays a layer attribute, so a
  rasterized layer below 100 % opacity differs slightly from the live layer
  (`ponytail:`).
- **GPU.** The compute shader has no curve. While the gamma is on, a type
  layer makes the GPU compositor return `UnsupportedText` and the caller falls
  back to the CPU, as it already does for channel-less type.

## Missing fonts

Qt decides what is installed, so the check lives in C++
(`fontIsInstalled` in `type_fonts.cpp`). A name is installed when it is the
bundled Liberation Sans, `QFontDatabase::hasFamily` knows it, or it is a
PostScript name whose stem (minus an `MT`/`PSMT`/`PS` suffix) is an installed
family's spaceless name. `QRawFont` is not used because fontconfig substitutes
any family. Results are cached for the process.

- **Alert.** `finishLoadingDocument` reads the document's type fonts
  (`pdf_type_fonts`, which already walks every type layer) and opens one
  non-blocking warning box listing the missing ones.
- **Badge.** The layer-row snapshot gains `missingFont`; the Layers panel
  draws a small warning triangle on the type card's corner.
- **Replace All Missing Fonts.** The C++ side passes the missing names to
  `type_replace_fonts`, which re-sets every type layer using one of them with
  Liberation Sans Regular (the Type tool's default) through
  `replace_type_layer`. A font override is rewritten too, so the replacement
  survives a style re-resolve. One "Replace All Missing Fonts" state; disabled
  when the active document has no missing font.
