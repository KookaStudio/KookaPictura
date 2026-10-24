//! The `Filter` -> ImageMagick mapping table and its coverage test.

use pictura_filters::FilterKind;

/// One row of the `Filter` -> ImageMagick mapping table. `im` is the equivalent
/// operator (documentation; the actual invocation lives with each test) or
/// `None` when there is no faithful equivalent.
struct Mapping {
    filter: &'static str,
    im: Option<&'static str>,
    tolerance: u8,
    note: &'static str,
}

const fn none(filter: &'static str, note: &'static str) -> Mapping {
    Mapping {
        filter,
        im: None,
        tolerance: 0,
        note,
    }
}

const MAPPING: &[Mapping] = &[
    Mapping {
        filter: "GaussianBlur",
        im: Some("-gaussian-blur 0x{sigma}  (sigma = radius / 3)"),
        tolerance: 0,
        note: "same 3-sigma separable Gaussian, clamp-to-edge; measured max delta 0",
    },
    Mapping {
        filter: "BoxBlur",
        im: Some("-statistic mean NxN  (N = 2*radius + 1)"),
        tolerance: 0,
        note: "same separable moving average, clamp-to-edge; measured max delta 0",
    },
    Mapping {
        filter: "Median",
        im: Some("-median {radius}"),
        tolerance: 0,
        note: "same (2r+1)^2 per-channel rank filter, clamp-to-edge; measured max delta 0",
    },
    Mapping {
        filter: "UnsharpMask",
        im: Some("-unsharp 0x{sigma}+{amount/100}+{threshold/255}"),
        tolerance: 6,
        note: "same blur-difference gain; IM amount is a fraction (100% = 1.0). IM's internal \
               blur differs slightly from its standalone -gaussian-blur: measured max delta 5 at \
               radius 3 / amount 150 / threshold 0",
    },
    Mapping {
        filter: "ShapeBlur",
        im: Some("-define convolve:scale=! -morphology Convolve {S}x{S}:{coverage weights}"),
        tolerance: 0,
        note: "same weighted convolution with the kernel rotated 180 degrees, clamp-to-edge, \
               fed Pictura's own coverage kernel, so it checks the convolution, not the shape \
               artwork; measured max delta 0 (heart radius 5, arrow radius 4)",
    },
    Mapping {
        filter: "MotionBlur",
        im: None,
        tolerance: 0,
        note: "IM -motion-blur builds a one-sided Gaussian line kernel; Pictura averages \
               symmetric uniform taps. Observed max delta 86 (vs -motion-blur 0x5+0)",
    },
    Mapping {
        filter: "RadialBlur",
        im: None,
        tolerance: 0,
        note: "ImageMagick 7 removed -radial-blur; -rotational-blur weights a different angle \
               profile. Observed max delta 58 (Spin 20 vs -rotational-blur 20)",
    },
    Mapping {
        filter: "Average",
        im: None,
        tolerance: 0,
        note: "trivial global region mean; no IM operator shares the window/border semantics. \
               Observed max delta 92 (vs -statistic mean 16x16)",
    },
    Mapping {
        filter: "Blur",
        im: Some("-gaussian-blur 0x0.7"),
        tolerance: 0,
        note: "a Gaussian at the fixed sigma 0.7 (radius 2.1); measured max delta 0",
    },
    Mapping {
        filter: "BlurMore",
        im: Some("-gaussian-blur 0x2"),
        tolerance: 1,
        note: "a Gaussian at the fixed sigma 2.0 (radius 6); IM's kernel at this sigma differs \
               by rounding: measured max delta 1",
    },
    Mapping {
        filter: "SurfaceBlur",
        im: None,
        tolerance: 0,
        note: "per-channel mean of the neighbours within the threshold; IM has no thresholded \
               mean operator. Observed max delta 71 (radius 3 / threshold 20 vs \
               -gaussian-blur 0x1)",
    },
    Mapping {
        filter: "SmartBlur",
        im: None,
        tolerance: 0,
        note: "A Gaussian over the neighbours whose every channel is within the threshold, plus \
               white edge marks; IM's `-selective-blur` gates on luma and draws no edges, so the \
               variant is no-equivalent. Guarded by the step, threshold, mode and quality unit \
               tests and the kernel digests",
    },
    Mapping {
        filter: "Sharpen",
        im: Some("-unsharp 0x1+0.5+0"),
        tolerance: 2,
        note: "Unsharp Mask at the fixed 50% / sigma 1; IM's internal blur differs slightly: \
               measured max delta 2",
    },
    Mapping {
        filter: "SharpenMore",
        im: Some("-unsharp 0x1+1+0"),
        tolerance: 3,
        note: "Unsharp Mask at the fixed 100% / sigma 1; IM's internal blur differs slightly: \
               measured max delta 3",
    },
    Mapping {
        filter: "SharpenEdges",
        im: None,
        tolerance: 0,
        note: "unsharp gated by a smoothstep of the Sobel edge strength; IM has no edge-gated \
               sharpen. Observed max delta 44 (vs -sharpen 0x1)",
    },
    Mapping {
        filter: "AddNoise",
        im: None,
        tolerance: 0,
        note: "RNG streams differ; same-seed determinism is the contract. Observed max delta 79 \
               (vs -attenuate 0.1 +noise Gaussian)",
    },
    Mapping {
        filter: "Despeckle",
        im: None,
        tolerance: 0,
        note: "IM -despeckle uses a different rank detector; observed max delta 13 (vs \
               -despeckle)",
    },
    Mapping {
        filter: "Maximum",
        im: Some("-morphology Dilate Square:{radius}"),
        tolerance: 0,
        note: "same (2r+1)^2 square grayscale dilate, clamp-to-edge; measured max delta 0 \
               (radius 2). NB: IM `Square:N` takes a *radius* (kernel diameter 2N+1), so the \
               faithful flag is `Square:{radius}`; the M7 plan's `N = 2*radius+1` would dilate \
               over a (4r+3)^2 footprint",
    },
    Mapping {
        filter: "Minimum",
        im: Some("-morphology Erode Square:{radius}"),
        tolerance: 0,
        note: "same (2r+1)^2 square grayscale erode, clamp-to-edge; measured max delta 0 \
               (radius 2). Same `Square:N` radius-vs-diameter note as Maximum",
    },
    Mapping {
        filter: "Offset",
        im: Some("-roll {+horizontal}{+vertical}  (wrap = true)"),
        tolerance: 0,
        note: "wrap = true is an exact integer roll; measured max delta 0 at (3,2) and (2,3). \
               wrap = false fills the exposed area with `background`, which -roll cannot do (it \
               always wraps); observed max delta 240 vs -roll",
    },
    Mapping {
        filter: "HighPass",
        im: None,
        tolerance: 0,
        note: "no single IM operator. A hand-built `\\( +clone -gaussian-blur 0x{sigma} \\) \
               -compose Mathematics -define compose:args=0,-1,1,0.5 -composite` re-implements \
               Pictura's formula and is within measured max delta 1 (radius 3.0, sigma 1.0), but \
               it is not an independent operator; guarded by the flat-field mid-gray test",
    },
    Mapping {
        filter: "Custom",
        im: Some("-convolve {kernel}, -define convolve:scale={sum(kernel)/scale}, -evaluate add {offset/255}%"),
        tolerance: 0,
        note: "same f64 5x5 convolution, clamp-to-edge. IM -convolve normalizes by the kernel \
               sum, so the matching scale is sum(kernel)/scale; measured max delta 0 (edge \
               kernel, scale 4 offset 8 and scale 9 offset -10)",
    },
    Mapping {
        filter: "Emboss",
        im: None,
        tolerance: 0,
        note: "IM -emboss is per-channel with a fixed diagonal kernel and no angle/height/amount; \
               Pictura is an angle-directed second difference on luma with an achromatic output. \
               No angle/radius/sigma matches: observed max delta 210 at angle 135 vs -emboss 0x1 \
               (best case 186 on the axis-aligned angles)",
    },
    Mapping {
        filter: "FindEdges",
        im: None,
        tolerance: 0,
        note: "IM -edge is a different detector and renders edges bright on dark; Pictura's Sobel \
               magnitude is inverted (dark on light). Observed max delta 255 vs -edge 1",
    },
    Mapping {
        filter: "Solarize",
        im: Some("-solarize 50%"),
        tolerance: 0,
        note: "same fixed 50% inversion curve (v >= 128 -> 255 - v); measured max delta 0",
    },
    none(
        "NtscColors",
        "no ImageMagick NTSC gamut-clamp operator; the YIQ approximation is pinned by the \
         pictura-color and video kernel tests",
    ),
    Mapping {
        filter: "Mosaic",
        im: Some("-filter box -resize {W/n}x{H/n}! -filter point -resize WxH!  (n = cell_size)"),
        tolerance: 0,
        note: "exact top-left block mean when the cell divides both dimensions; measured max delta 0 \
               at cell 4 on 16x16. For non-divisor cells IM's resize window is offset from Pictura's \
               blocks (observed max delta 80..136 at cell 3/5/6/7), so the differential uses cell 4",
    },
    Mapping {
        filter: "Crystallize",
        im: None,
        tolerance: 0,
        note: "seeded Voronoi cells have no ImageMagick operator; closest approximations -kuwahara 4 \
               (observed max delta 118) and -paint 4 (157). Guarded by seed determinism and the \
               flat-field identity property",
    },
    Mapping {
        filter: "Facet",
        im: None,
        tolerance: 0,
        note: "similar-neighbor banded 3x3 mean; IM -statistic mean 3x3 is the closest (observed max \
               delta 76; -kuwahara 1 gives 144). Guarded by flat-field identity and gradient flattening",
    },
    Mapping {
        filter: "Fragment",
        im: None,
        tolerance: 0,
        note: "clamp-anchored sliding 2x2 mean; -statistic mean 2x2 uses a different window/edge rule \
               (observed max delta 85; -kuwahara 1 gives 59). Guarded by the 4-tap known-value test",
    },
    Mapping {
        filter: "Mezzotint",
        im: None,
        tolerance: 0,
        note: "seeded procedural dot/line pattern; IM -threshold 50% and -ordered-dither are different \
               screens (observed max delta 255). Guarded by seed determinism, binarity and achromatic \
               output",
    },
    Mapping {
        filter: "Pointillize",
        im: None,
        tolerance: 0,
        note: "seeded local-color dots over background; IM -spread 2 displaces pixels instead of \
               drawing dots (observed max delta 240; -kuwahara 2 gives 232). Guarded by seed \
               determinism and the source/background color set",
    },
    Mapping {
        filter: "ColorHalftone",
        im: None,
        tolerance: 0,
        note: "per-channel rotated screen has no ImageMagick operator; -ordered-dither o8x8 / h4x4a are \
               fixed orthogonal screens (observed max delta 255). Guarded by determinism and binarity",
    },
    Mapping {
        filter: "Twirl",
        im: None,
        tolerance: 0,
        note: "IM -swirl uses a smooth falloff and radius = min(w,h)/2; Pictura twirls with a linear \
               falloff over max(cx,cy). Best measured max delta 124 (Twirl +45 vs -swirl 45, same \
               sign); -90 vs -swirl 90 gives 170. Guarded by the zero no-op and rotation tests",
    },
    Mapping {
        filter: "Pinch",
        im: None,
        tolerance: 0,
        note: "IM -implode amount is a fraction (not a percent); positive implodes, negative explodes. \
               Pictura's linear radial remap is not IM's implode falloff. Best measured max delta 61 \
               (Pinch 50 vs -implode 0.5); Pinch -50 vs -implode -0.5 gives 84",
    },
    Mapping {
        filter: "Spherize",
        im: None,
        tolerance: 0,
        note: "closest is -implode {amount/100}; Pictura's arc-length sphere map (Normal / \
               HorizontalOnly / VerticalOnly) is not IM's implode falloff. Best measured max delta 159 \
               (Spherize 50 Normal vs -implode 0.5); -50 gives 128",
    },
    Mapping {
        filter: "Ripple",
        im: None,
        tolerance: 0,
        note: "IM -wave displaces one axis with a sine in x and pads the canvas (background fill), \
               while Pictura ripple displaces both axes (dx ~ sin y, dy ~ sin x) with clamp-to-edge \
               and a fixed period per size. Measured max delta 255 / mean 104 (Ripple 100 Medium vs \
               -wave 10x16 cropped to 16x16)",
    },
    Mapping {
        filter: "Wave",
        im: None,
        tolerance: 0,
        note: "IM -wave is a single unseeded sine along one axis; Pictura sums seeded generators with \
               random phase/period/amplitude and an axis-wise scale. Measured max delta 255 / mean 101 \
               (1 generator sine, amp 20, wavelength 10, seed 42 vs -wave 20x10 cropped)",
    },
    Mapping {
        filter: "PolarCoordinates",
        im: None,
        tolerance: 0,
        note: "IM -distort Polar/DePolar use a different angle origin (180 deg off) and \
               pixel-center/radius anchor plus IM's own resampling. Best measured: RectangularToPolar \
               vs `-distort Polar 0` max delta 189 / mean 58.3; PolarToRectangular vs \
               `-distort DePolar 0` max 194 / mean 58.9 (the crossed directions give max 240/241). Guarded \
               by the remap/no-op property tests",
    },
    Mapping {
        filter: "Shear",
        im: None,
        tolerance: 0,
        note: "IM `-shear {angle}x0` is a whole-canvas horizontal shear that background-fills the \
               expanded canvas (default black), while Pictura shifts each row by a smooth spline through the \
               control points with clamp/wrap and expands nothing. The curve's slope `[(-1,-0.5),(1,0.5)]` = \
               atan(0.5) = 26.565 deg, which IM's sign convention runs the other way; best measured \
               `-shear -26.565x0 -crop 16x16+4+0 +repage` vs RepeatEdgePixels max 255 / mean 18.4 \
               (WrapAround mean 23.4). Guarded by the zero-curve no-op and fill property tests",
    },
    Mapping {
        filter: "Displace",
        im: Some("MAP -compose displace -define compose:args={1.275*h}x{1.275*v} -composite"),
        tolerance: 8,
        note: "Repeat Edge Pixels: the same bilinear lookup with red across and green down; IM \
               centres on 127.5, so measured max delta 5 / mean 0.5 at 4x4 and 6x-5 (9 at 8x-6). Wrap Around diverges at the \
               seam (IM blends across it), so only Repeat Edge Pixels is diffed",
    },
    Mapping {
        filter: "ZigZag",
        im: None,
        tolerance: 0,
        note: "IM `-swirl` uses a smooth falloff about min(w,h)/2; Pictura's cosine radial profile is \
               pinned to zero at the edge with `ridges` reversals. Best measured `-swirl 50` vs amount \
               80 / ridges 5 / AroundCenter max 170 / mean 14.3 (`-swirl 80` mean 14.9, `-swirl -80` \
               mean 18.8, `-implode 0.8` mean 18.6). Guarded by the zero no-op and style tests",
    },
    Mapping {
        filter: "OceanRipple",
        im: None,
        tolerance: 0,
        note: "IM `-wave` is an unseeded single-axis sine that pads the canvas; Pictura refracts \
               through the slope of seeded 2-D value noise with clamp-to-edge, which no single `-wave` \
               reproduces. Guarded by seed determinism, the zero-magnitude no-op and the reach test",
    },
    Mapping {
        filter: "DustAndScratches",
        im: None,
        tolerance: 0,
        note: "IM `-statistic median NxN` is an ungated rank filter; Pictura replaces a pixel only \
               when it differs from the local median by more than `threshold`, and no IM operator \
               exposes that gate. Guarded by the speck-removal and determinism unit tests",
    },
    Mapping {
        filter: "ReduceNoise",
        im: None,
        tolerance: 0,
        note: "An edge-preserving bilateral denoise per plane, then a luminance-holding chroma \
               blur, an unsharp pass and an 8x8 deblock; ImageMagick has no bilateral or \
               Reduce-Noise operator that matches. Guarded by the monotonic-Strength, \
               Preserve-Details, chroma-variance, deblock and determinism unit tests",
    },
    Mapping {
        filter: "Extrude",
        im: None,
        tolerance: 0,
        note: "No ImageMagick extrusion renderer; the block/face geometry plus the solid-front, \
               level-based and mask-incomplete options have no operator. Guarded by the \
               changed-image and determinism unit test",
    },
    Mapping {
        filter: "Tiles",
        im: None,
        tolerance: 0,
        note: "No IM tiled-offset-with-fill operator; `-roll` and `-spread` neither offset a fixed \
               grid nor fill the gaps with the foreground/background choice. Guarded by the \
               determinism unit test",
    },
    Mapping {
        filter: "TraceContour",
        im: None,
        tolerance: 0,
        note: "No IM per-channel level-crossing contour operator; `-edge`, `-morphology` and \
               `-threshold` are different detectors. Guarded by the contour unit tests",
    },
    Mapping {
        filter: "Wind",
        im: None,
        tolerance: 0,
        note: "No IM horizontal-streak operator; `-motion-blur`, `-spread` and `-wave` displace \
               pixels differently. Guarded by the determinism unit test",
    },
    Mapping {
        filter: "SmartSharpen",
        im: None,
        tolerance: 0,
        note: "A deconvolution of a sharp-core + halo blur model; IM's `-unsharp` is a plain \
               blur difference and has no deconvolution operator, so the variant is \
               no-equivalent. Guarded by the core-inverse, halo-convergence, remove-path and \
               determinism unit tests",
    },
    Mapping {
        filter: "LensBlur",
        im: None,
        tolerance: 0,
        note: "A depth-map-driven aperture convolution: ImageMagick has no depth-aware iris \
               blur, no circle-of-confusion ramp, no specular bloom and no seeded grain. \
               Guarded by the focus, ramp, iris, specular and noise unit tests",
    },
    Mapping {
        filter: "FieldBlur",
        im: None,
        tolerance: 0,
        note: "A pin-interpolated variable blur: ImageMagick has no field of per-pixel radii \
               driven by weighted pin amounts. Guarded by the gradient, outside-pin, \
               no-op, validation and determinism unit tests",
    },
    Mapping {
        filter: "IrisBlur",
        im: None,
        tolerance: 0,
        note: "Elliptical sharp islands with a feathered fade into a variable blur: \
               ImageMagick has no per-pixel radius field shaped by superellipse pins. \
               Guarded by the core, fade-ramp, falloff, two-island, no-op, validation and \
               determinism unit tests",
    },
    Mapping {
        filter: "TiltShift",
        im: None,
        tolerance: 0,
        note: "A sharp band fading into a variable blur with a radial/tangential distortion \
               smear: ImageMagick has no per-pixel radius field shaped by band pins. Guarded \
               by the focus, fade-ramp, rotation, two-band, distortion, no-op, validation and \
               determinism unit tests",
    },
    Mapping {
        filter: "PathBlur",
        im: None,
        tolerance: 0,
        note: "Motion blur along a field blended from drawn Catmull-Rom paths, gathered along \
               its streamlines: ImageMagick's -motion-blur takes one fixed angle and length. \
               Guarded by the row/stripe, trailing, strobe, taper, rear sync, curved-field, \
               nearest-path, no-op, validation and determinism unit tests",
    },
    Mapping {
        filter: "SpinBlur",
        im: None,
        tolerance: 0,
        note: "Rotational blur along arcs round each spin's rotation centre inside a feathered \
               ellipse, with strobe flashes: ImageMagick's -rotational-blur turns the whole \
               image round one centre without a mask. Guarded by the ellipse, circles/spokes, \
               pivot, strobe, validation, alpha, no-op and determinism unit tests",
    },
    Mapping {
        filter: "BlurGallery",
        im: None,
        tolerance: 0,
        note: "Field, Iris and Tilt-Shift radius fields combined by their maximum into one \
               variable blur: no ImageMagick operator. Guarded by the single-effect parity, \
               combine, no-op and determinism unit tests",
    },
    none(
        "Diffuse",
        "ImageMagick -spread displaces pixels rather than applying Pictura's mode-dependent \
         diffusion; guarded by the ported contract and scenario tests",
    ),
    none(
        "GlowingEdges",
        "ImageMagick -edge is a different detector and has no glow width, brightness or \
         smoothness controls; guarded by the ported contract and scenario tests",
    ),
    none(
        "Liquify",
        "ImageMagick has no mesh-warp operator; guarded by the liquify tests",
    ),
    none(
        "VanishingPoint",
        "ImageMagick has no perspective-plane grid; guarded by the vanishing_point tests",
    ),
    none(
        "AdaptiveWideAngle",
        "ImageMagick has no constraint-solved projection; guarded by the adaptive_wide_angle \
         tests",
    ),
    none(
        "Glass",
        "no ImageMagick texture-refraction operator with Pictura's surface smoothing and \
         scaling; guarded by the glass tests",
    ),
    none(
        "DiffuseGlow",
        "no ImageMagick glow-and-grain composite with the dialog's graininess, glow and clear \
         amounts; guarded by the diffuse-glow tests",
    ),
    none(
        "Clouds",
        "no ImageMagick operator reproduces the seeded fractal cloud field and its starker \
         threshold; guarded by the seed and determinism tests",
    ),
    none(
        "DifferenceClouds",
        "no ImageMagick operator reproduces the seeded difference-cloud field; guarded by the \
         seed and determinism tests",
    ),
    none(
        "Fibers",
        "no ImageMagick operator reproduces the seeded fiber field and its variance/strength \
         controls; guarded by the seed and determinism tests",
    ),
    none(
        "PictureFrame",
        "no ImageMagick decorative-frame compositor; guarded by the picture-frame tests",
    ),
    none(
        "LensFlare",
        "no ImageMagick lens-flare compositor with Pictura's lens presets; guarded by the \
         lens-flare tests",
    ),
    none(
        "Lighting",
        "no ImageMagick lighting-effects renderer over an editable light rig; guarded by the \
         lighting tests",
    ),
    none(
        "Cutout",
        "no ImageMagick cutout operator with levels, edge simplicity and edge fidelity; guarded \
         by the ported contract and scenario tests",
    ),
    none(
        "FilmGrain",
        "ImageMagick +noise adds grain but has no film-grain highlight and intensity model; \
         guarded by the ported contract and scenario tests",
    ),
    none(
        "NeonGlow",
        "no ImageMagick neon-glow operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "PosterEdges",
        "no ImageMagick poster-edges operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "PaintDaubs",
        "no ImageMagick paint-daubs operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "PaletteKnife",
        "no ImageMagick palette-knife operator; guarded by the ported contract and scenario \
         tests",
    ),
    none(
        "PlasticWrap",
        "no ImageMagick plastic-wrap renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "Sponge",
        "no ImageMagick sponge renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "ColoredPencil",
        "no ImageMagick colored-pencil renderer; guarded by the ported contract and scenario \
         tests",
    ),
    none(
        "DryBrush",
        "no ImageMagick dry-brush operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "Fresco",
        "no ImageMagick fresco renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "RoughPastels",
        "no ImageMagick rough-pastels renderer; guarded by the ported contract and scenario \
         tests",
    ),
    none(
        "SmudgeStick",
        "no ImageMagick smudge-stick operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "Underpainting",
        "no ImageMagick underpainting renderer; guarded by the ported contract and scenario \
         tests",
    ),
    none(
        "Watercolor",
        "no ImageMagick watercolor renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "AccentedEdges",
        "no ImageMagick accents/edge operator with the dialog's width, brightness and \
         smoothness; guarded by the ported contract and scenario tests",
    ),
    none(
        "AngledStrokes",
        "no ImageMagick directional-stroke operator; guarded by the ported contract and \
         scenario tests",
    ),
    none(
        "Crosshatch",
        "no ImageMagick crosshatch operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "DarkStrokes",
        "no ImageMagick dark-strokes operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "InkOutlines",
        "no ImageMagick ink-outlines operator; guarded by the ported contract and scenario \
         tests",
    ),
    none(
        "Spatter",
        "no ImageMagick spatter operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "SprayedStrokes",
        "no ImageMagick sprayed-strokes operator; guarded by the ported contract and scenario \
         tests",
    ),
    none(
        "SumiE",
        "no ImageMagick sumi-e operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "BasRelief",
        "ImageMagick -emboss is a fixed diagonal kernel with no detail, smoothness or light \
         control; guarded by the ported contract and scenario tests",
    ),
    none(
        "ChalkCharcoal",
        "no ImageMagick chalk-and-charcoal renderer; guarded by the ported contract and scenario \
         tests",
    ),
    none(
        "Charcoal",
        "no ImageMagick charcoal renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "Chrome",
        "no ImageMagick chrome renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "ConteCrayon",
        "no ImageMagick conte-crayon renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "GraphicPen",
        "no ImageMagick graphic-pen operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "HalftonePattern",
        "no ImageMagick halftone-pattern operator with size, contrast and pattern controls; \
         guarded by the ported contract and scenario tests",
    ),
    none(
        "NotePaper",
        "no ImageMagick note-paper renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "Photocopy",
        "no ImageMagick photocopy operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "Plaster",
        "no ImageMagick plaster renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "Reticulation",
        "no ImageMagick reticulation renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "Stamp",
        "no ImageMagick stamp renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "TornEdges",
        "no ImageMagick torn-edges operator; guarded by the ported contract and scenario tests",
    ),
    none(
        "WaterPaper",
        "no ImageMagick water-paper renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "Craquelure",
        "no ImageMagick crackle renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "Grain",
        "ImageMagick +noise is not the seeded grain-type model; guarded by the ported contract \
         and scenario tests",
    ),
    none(
        "MosaicTiles",
        "no ImageMagick mosaic-tile renderer with grout and lighten controls; guarded by the \
         ported contract and scenario tests",
    ),
    none(
        "Patchwork",
        "no ImageMagick patchwork renderer; guarded by the ported contract and scenario tests",
    ),
    none(
        "StainedGlass",
        "no ImageMagick stained-glass renderer; guarded by the ported contract and scenario \
         tests",
    ),
    none(
        "Texturizer",
        "no ImageMagick texturizer with the dialog's surface, relief and light controls; guarded \
         by the ported contract and scenario tests",
    ),
    none(
        "OilPaint",
        "no ImageMagick oil-paint brush-kernel renderer; guarded by the oil-paint tests",
    ),
    none(
        "HdrToning",
        "no ImageMagick HDR tone-mapping operator with Pictura's local adaptation; guarded by \
         the HDR toning tests",
    ),
];

#[test]
fn mapping_table_holds_one_row_per_variant() {
    let rows: Vec<&str> = MAPPING.iter().map(|mapping| mapping.filter).collect();
    let unique: std::collections::BTreeSet<&str> = rows.iter().copied().collect();
    assert_eq!(unique.len(), rows.len(), "a mapping row is repeated");
    let expected: std::collections::BTreeSet<&str> =
        FilterKind::ALL.iter().map(|kind| kind.name()).collect();
    assert_eq!(unique, expected, "every Filter variant needs one row");
}

#[test]
fn every_mapping_row_states_an_oracle_or_states_none() {
    for mapping in MAPPING {
        assert!(!mapping.note.is_empty(), "{}: empty note", mapping.filter);
        match mapping.im {
            Some(operator) => {
                assert!(!operator.is_empty(), "{}: empty oracle", mapping.filter);
                assert!(
                    mapping.tolerance <= 8,
                    "{}: tolerance out of range",
                    mapping.filter
                );
            }
            None => assert_eq!(
                mapping.tolerance, 0,
                "{}: no equivalent must not set tolerance",
                mapping.filter
            ),
        }
    }
}
