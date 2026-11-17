//! The Layer Style dialog's fields: for each of the ten effects, the dialog
//! key, the `lfx2` descriptor key, how its number maps onto the descriptor, and
//! CS6's default (`docs/05-layers/layer-styles.md`; the decoders' defaults
//! where the spec marks a value inferred).

/// How a field's number maps onto its descriptor value.
#[derive(Clone, Copy)]
pub(super) enum Kind {
    Bool,
    /// A `UntF` with the given unit, clamped to `[min, max]`.
    Unit(&'static [u8; 4], f64, f64),
    /// An `RGBC` object, packed `0xRRGGBB`.
    Color,
    /// A `BlnM` enum, indexed in [`BlendMode::LAYER_MODES`].
    Blend,
    /// An enum of type `kind`, indexed in `options`.
    Choice(&'static [u8], &'static [&'static [u8]]),
    /// The colour of the gradient's first (`false`) or last (`true`) stop.
    GradientStop(bool),
}

pub(super) struct Field {
    pub name: &'static str,
    pub key: &'static [u8],
    pub kind: Kind,
    pub default: f64,
}

const fn field(name: &'static str, key: &'static [u8], kind: Kind, default: f64) -> Field {
    Field {
        name,
        key,
        kind,
        default,
    }
}

const PCT: Kind = Kind::Unit(b"#Prc", 0.0, 100.0);
const PX: Kind = Kind::Unit(b"#Pxl", 0.0, 250.0);
const DIST: Kind = Kind::Unit(b"#Pxl", 0.0, 30_000.0);
const ANGLE: Kind = Kind::Unit(b"#Ang", -360.0, 360.0);

const fn mode(index: usize) -> f64 {
    index as f64
}
const NORMAL: f64 = mode(0);
const MULTIPLY: f64 = mode(3);
const SCREEN: f64 = mode(8);

const TECHNIQUE: Kind = Kind::Choice(b"BETE", &[b"SfBL", b"PrBL"]);

const DROP_SHADOW: &[Field] = &[
    field("mode", b"Md  ", Kind::Blend, MULTIPLY),
    field("color", b"Clr ", Kind::Color, 0x000000 as f64),
    field("opacity", b"Opct", PCT, 75.0),
    field("useGlobalLight", b"uglg", Kind::Bool, 1.0),
    field("angle", b"lagl", ANGLE, 120.0),
    field("distance", b"Dstn", DIST, 5.0),
    field("spread", b"Ckmt", Kind::Unit(b"#Pxl", 0.0, 100.0), 0.0),
    field("size", b"blur", PX, 5.0),
    field("noise", b"Nose", PCT, 0.0),
    field("antiAlias", b"AntA", Kind::Bool, 0.0),
    field("knocksOut", b"layerConceals", Kind::Bool, 1.0),
];

const INNER_SHADOW: &[Field] = &[
    field("mode", b"Md  ", Kind::Blend, MULTIPLY),
    field("color", b"Clr ", Kind::Color, 0x000000 as f64),
    field("opacity", b"Opct", PCT, 75.0),
    field("useGlobalLight", b"uglg", Kind::Bool, 1.0),
    field("angle", b"lagl", ANGLE, 120.0),
    field("distance", b"Dstn", DIST, 5.0),
    field("choke", b"Ckmt", Kind::Unit(b"#Pxl", 0.0, 100.0), 0.0),
    field("size", b"blur", PX, 5.0),
    field("noise", b"Nose", PCT, 0.0),
    field("antiAlias", b"AntA", Kind::Bool, 0.0),
];

const OUTER_GLOW: &[Field] = &[
    field("mode", b"Md  ", Kind::Blend, SCREEN),
    field("color", b"Clr ", Kind::Color, 0xFFFFBE as f64),
    field("opacity", b"Opct", PCT, 75.0),
    field("technique", b"GlwT", TECHNIQUE, 0.0),
    field("spread", b"Ckmt", Kind::Unit(b"#Pxl", 0.0, 100.0), 0.0),
    field("size", b"blur", PX, 5.0),
    field("noise", b"Nose", PCT, 0.0),
    field("range", b"Inpr", PCT, 50.0),
    field("jitter", b"ShdN", PCT, 0.0),
    field("antiAlias", b"AntA", Kind::Bool, 0.0),
];

const INNER_GLOW: &[Field] = &[
    field("mode", b"Md  ", Kind::Blend, SCREEN),
    field("color", b"Clr ", Kind::Color, 0xFFFFBE as f64),
    field("opacity", b"Opct", PCT, 75.0),
    field("technique", b"GlwT", TECHNIQUE, 0.0),
    field(
        "source",
        b"glwS",
        Kind::Choice(b"IGSr", &[b"SrcC", b"SrcE"]),
        1.0,
    ),
    field("choke", b"Ckmt", Kind::Unit(b"#Pxl", 0.0, 100.0), 0.0),
    field("size", b"blur", PX, 5.0),
    field("noise", b"Nose", PCT, 0.0),
    field("range", b"Inpr", PCT, 50.0),
    field("jitter", b"ShdN", PCT, 0.0),
    field("antiAlias", b"AntA", Kind::Bool, 0.0),
];

const BEVEL: &[Field] = &[
    field(
        "style",
        b"bvlS",
        Kind::Choice(
            b"BESl",
            &[b"OtrB", b"InrB", b"Embs", b"PlEb", b"strokeEmboss"],
        ),
        1.0,
    ),
    field(
        "technique",
        b"bvlT",
        Kind::Choice(b"bvlT", &[b"SfBL", b"PrBL", b"Slmt"]),
        0.0,
    ),
    field("depth", b"srgR", Kind::Unit(b"#Prc", 1.0, 1000.0), 100.0),
    field(
        "direction",
        b"bvlD",
        Kind::Choice(b"BESs", &[b"In  ", b"Out "]),
        0.0,
    ),
    field("size", b"blur", PX, 5.0),
    field("soften", b"Sftn", Kind::Unit(b"#Pxl", 0.0, 16.0), 0.0),
    field("useGlobalLight", b"uglg", Kind::Bool, 1.0),
    field("angle", b"lagl", ANGLE, 120.0),
    field("altitude", b"Lald", Kind::Unit(b"#Ang", 0.0, 90.0), 30.0),
    field("highlightMode", b"hglM", Kind::Blend, SCREEN),
    field("highlightColor", b"hglC", Kind::Color, 0xFFFFFF as f64),
    field("highlightOpacity", b"hglO", PCT, 75.0),
    field("shadowMode", b"sdwM", Kind::Blend, MULTIPLY),
    field("shadowColor", b"sdwC", Kind::Color, 0x000000 as f64),
    field("shadowOpacity", b"sdwO", PCT, 75.0),
    field("antiAlias", b"antialiasGloss", Kind::Bool, 1.0),
    field("edgeContourOn", b"useShape", Kind::Bool, 0.0),
    field("edgeRange", b"Inpr", Kind::Unit(b"#Prc", 1.0, 100.0), 50.0),
    field("edgeAntiAlias", b"AntA", Kind::Bool, 0.0),
];

const SATIN: &[Field] = &[
    field("mode", b"Md  ", Kind::Blend, MULTIPLY),
    field("color", b"Clr ", Kind::Color, 0x000000 as f64),
    field("opacity", b"Opct", PCT, 50.0),
    field("angle", b"lagl", ANGLE, 19.0),
    field("distance", b"Dstn", PX, 11.0),
    field("size", b"blur", PX, 14.0),
    field("antiAlias", b"AntA", Kind::Bool, 0.0),
    field("invert", b"Invr", Kind::Bool, 0.0),
];

const COLOR_OVERLAY: &[Field] = &[
    field("mode", b"Md  ", Kind::Blend, NORMAL),
    field("color", b"Clr ", Kind::Color, 0xFF0000 as f64),
    field("opacity", b"Opct", PCT, 100.0),
];

const GRADIENT_OVERLAY: &[Field] = &[
    field("mode", b"Md  ", Kind::Blend, NORMAL),
    field("opacity", b"Opct", PCT, 100.0),
    field("from", b"Grad", Kind::GradientStop(false), 0x000000 as f64),
    field("to", b"Grad", Kind::GradientStop(true), 0xFFFFFF as f64),
    field("reverse", b"Rvrs", Kind::Bool, 0.0),
    field(
        "style",
        b"Type",
        Kind::Choice(b"GrdT", &[b"Lnr ", b"Rdl ", b"Angl", b"Rflc", b"Dmnd"]),
        0.0,
    ),
    field("align", b"Algn", Kind::Bool, 1.0),
    field("angle", b"Angl", ANGLE, 90.0),
    field("scale", b"Scl ", Kind::Unit(b"#Prc", 10.0, 150.0), 100.0),
    field("dither", b"Dthr", Kind::Bool, 0.0),
];

const PATTERN_OVERLAY: &[Field] = &[
    field("mode", b"Md  ", Kind::Blend, NORMAL),
    field("opacity", b"Opct", PCT, 100.0),
    field("scale", b"Scl ", Kind::Unit(b"#Prc", 1.0, 1000.0), 100.0),
    field("link", b"Algn", Kind::Bool, 1.0),
];

const STROKE: &[Field] = &[
    field("size", b"Sz  ", Kind::Unit(b"#Pxl", 1.0, 250.0), 3.0),
    field(
        "position",
        b"Styl",
        Kind::Choice(b"FStl", &[b"OutF", b"InsF", b"CtrF"]),
        0.0,
    ),
    field("mode", b"Md  ", Kind::Blend, NORMAL),
    field("opacity", b"Opct", PCT, 100.0),
    field("color", b"Clr ", Kind::Color, 0x000000 as f64),
];

/// One effect: its dialog key prefix, its `lfx2` item key and class, its fields.
pub(super) struct Effect {
    pub name: &'static str,
    pub key: &'static [u8],
    pub fields: &'static [Field],
}

/// CS6's list order, which is also the dialog's.
pub(super) const EFFECTS: &[Effect] = &[
    Effect {
        name: "bevel",
        key: b"ebbl",
        fields: BEVEL,
    },
    Effect {
        name: "stroke",
        key: b"FrFX",
        fields: STROKE,
    },
    Effect {
        name: "innerShadow",
        key: b"IrSh",
        fields: INNER_SHADOW,
    },
    Effect {
        name: "innerGlow",
        key: b"IrGl",
        fields: INNER_GLOW,
    },
    Effect {
        name: "satin",
        key: b"ChFX",
        fields: SATIN,
    },
    Effect {
        name: "colorOverlay",
        key: b"SoFi",
        fields: COLOR_OVERLAY,
    },
    Effect {
        name: "gradientOverlay",
        key: b"GrFl",
        fields: GRADIENT_OVERLAY,
    },
    Effect {
        name: "patternOverlay",
        key: b"patternFill",
        fields: PATTERN_OVERLAY,
    },
    Effect {
        name: "outerGlow",
        key: b"OrGl",
        fields: OUTER_GLOW,
    },
    Effect {
        name: "dropShadow",
        key: b"DrSh",
        fields: DROP_SHADOW,
    },
];
