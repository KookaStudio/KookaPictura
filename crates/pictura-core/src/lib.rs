//! Core document types for Kooka Pictura.
//!
//! M0 scope is deliberately tiny: an 8-bit RGB/gray composite image. The full
//! layer/channel/mask model is specified in `docs/01-architecture/document-model.md`
//! and is **not** implemented here yet.

mod advanced_blending;
mod annotations;
pub mod blur;
mod crs;
mod guides;
pub mod nonseparable;
pub mod path;
mod plane;
mod samples;
pub mod shape;
mod text_render;
mod type_tool;
mod vector;
pub use advanced_blending::{BlendIf, Knockout};
pub use annotations::{
    Annotations, CountGroup, Marker, MarkerKind, Measurement, Ruler, COUNT_LABEL_SIZE_MAX,
    COUNT_LABEL_SIZE_MIN, COUNT_MARKER_SIZE_MAX, COUNT_MARKER_SIZE_MIN, DEFAULT_COUNT_COLOR,
    MAX_COLOR_SAMPLERS,
};
pub use crs::{CrsSettings, PicturaRawSettings};
pub use guides::{guide_near, Guide, GuideOrientation};
pub use plane::{fresh_stamp, Plane};
pub use samples::{Sample, Samples};
pub use text_render::{
    layout_lines, FontPolicy, GlyphMask, LayoutLine, LayoutParams, PlacedGlyph, RasterRequest,
    Rasterizer, ShapedGlyph, TextAlign, TextLayout, TextProvenance,
};
pub use type_tool::{
    AntiAlias, CharacterAttrs, CharacterOverrides, CharacterStyle, Composer, Justify, KerningMode,
    Leading, ParagraphAttrs, ParagraphOverrides, ParagraphStyle, ResolvedStyle, StyleError,
    StyleOverrides, TextStyle, TextStyleSheet, TypeSpec, TypeTool, BASIC_PARAGRAPH,
    DEFAULT_LANGUAGE,
};
pub use vector::{VectorFillRule, VectorMask, VectorSubpath};

/// PSD color modes (`header.color_mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Bitmap,
    Grayscale,
    Indexed,
    Rgb,
    Cmyk,
    Multichannel,
    Duotone,
    Lab,
}

impl ColorMode {
    /// Number of color channels the composite carries in PSD.
    ///
    /// Extra alpha/spot/selection channels are additional; Multichannel has no
    /// color channels at all (its count is header-authoritative).
    pub fn color_channels(self) -> u8 {
        match self {
            ColorMode::Bitmap | ColorMode::Grayscale | ColorMode::Indexed | ColorMode::Duotone => 1,
            ColorMode::Rgb | ColorMode::Lab => 3,
            ColorMode::Cmyk => 4,
            ColorMode::Multichannel => 0,
        }
    }
}

/// Bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitDepth {
    One,
    Eight,
    Sixteen,
    ThirtyTwo,
}

/// PSD image-data compression method (the `image_data.compression` word).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    #[default]
    Rle,
    Raw,
    Zip,
    ZipPrediction,
}

impl Compression {
    /// The PSD compression word (0/1/2/3).
    pub fn to_code(self) -> u16 {
        match self {
            Compression::Raw => 0,
            Compression::Rle => 1,
            Compression::Zip => 2,
            Compression::ZipPrediction => 3,
        }
    }

    /// Parse a PSD compression word; `None` for an unknown code.
    pub fn from_code(code: u16) -> Option<Compression> {
        Some(match code {
            0 => Compression::Raw,
            1 => Compression::Rle,
            2 => Compression::Zip,
            3 => Compression::ZipPrediction,
            _ => return None,
        })
    }
}

/// A planar, row-major pixel buffer holding one sample type per channel.
///
/// `data.len() == width * height * channels`. Planar means channel `c` for the
/// whole image comes first, matching the PSD image-data layout. A bare
/// `PixelBuffer` is `PixelBuffer<u8>`, so 8-bit code is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelBuffer<T = u8> {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub data: Plane<T>,
}

impl<T: Clone + Default> PixelBuffer<T> {
    pub fn new(width: u32, height: u32, channels: u8) -> Self {
        Self {
            width,
            height,
            channels,
            data: vec![T::default(); width as usize * height as usize * channels as usize].into(),
        }
    }
}

impl<T> PixelBuffer<T> {
    /// Number of pixels (not samples).
    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

/// A minimal document: dimensions, mode, depth, one composite image, and a
/// layer tree (bottom-first, matching PSD z-order on disk).
///
/// Cloning is a refcount bump over the pixel planes: the copy shares every
/// plane until one of them is written, and only that plane forks ([`Plane`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub width: u32,
    pub height: u32,
    pub mode: ColorMode,
    pub depth: BitDepth,
    /// The header color mode of the file this document was read from, when that
    /// mode was normalized to the working mode on read. `None` for a Grayscale
    /// or RGB file and for a constructed document.
    pub source_mode: Option<ColorMode>,
    /// The header bit depth of the file this document was read from when it was
    /// not 8-bit. `None` for an 8-bit file, a depth-1 Bitmap file (whose mode
    /// `source_mode` records), and a constructed document; `Some(16/32)` for a
    /// 16/32-bit file of any mode, including a converted CMYK/Lab read. A
    /// Grayscale/RGB 16/32-bit read also retains native planes and re-emits this
    /// depth on save; a converted mode records it for the notice but retains no
    /// samples and still saves 8-bit.
    pub source_depth: Option<BitDepth>,
    /// Retained source planes of the composite color channels and the document
    /// extra channels, re-emitted on save when the plane is unchanged. Set for a
    /// 16/32-bit Grayscale or RGB read (native-depth samples) and for an 8-bit
    /// Lab read (the Lab color planes, so an unedited Lab file saves exactly).
    /// `None` for an 8-bit RGB/Grayscale read, a constructed document, and a
    /// 16/32-bit converted mode.
    pub source_planes: Option<SourcePlanes>,
    /// The 768-byte Indexed palette (256 red, then green, then blue) read from an
    /// Indexed file, retained so an unchanged document can re-emit it. `None` for
    /// every other mode, an RGB/Grayscale read, and a constructed document.
    /// Deliberately separate from `color_mode_data`, which stays cleared on an
    /// Indexed read (normalization consumes the palette).
    pub source_palette: Option<[u8; 768]>,
    /// The embedded ICC profile bytes of the file this document was read from,
    /// when it was converted to the sRGB working space on read. `None` for a
    /// file with no decodable non-sRGB profile and for a constructed document.
    /// Deliberately not re-emitted on save: the profile is removed from
    /// `image_resources` so the converted sRGB pixels are not mis-tagged.
    pub source_icc: Option<Vec<u8>>,
    /// The ICC bytes of the working profile the stored composite and layer color
    /// numbers are in. `None` means the sRGB working space. Set by an
    /// assign/convert command, or by a Preserve incoming-policy read that kept the
    /// embedded non-sRGB profile; distinct from `source_icc`, which records a
    /// profile a read converted away.
    pub document_icc: Option<Vec<u8>>,
    pub composite: PixelBuffer,
    /// True when the file carried a merged composite image-data section. False
    /// when the file ended after the layer section (maximize-compatibility off),
    /// in which case `composite` is a zero-filled placeholder, not authoritative.
    pub merged_composite_present: bool,
    /// Compression observed for the composite image-data section on read; RLE for
    /// a constructed document. Re-emitted on save to preserve the source encoding.
    pub composite_compression: Compression,
    /// Compression observed for the first engine-encoded layer channel (layer
    /// color channels and the raster mask) on read; RLE for a constructed
    /// document. Re-emitted on save; a document mixing kinds normalizes to the
    /// first seen.
    pub layer_compression: Compression,
    /// True when the document was read from a version-2 PSB container; false for
    /// a PSD and for new/blank documents. Selects the PSB container and its
    /// widened length fields on re-save.
    pub is_psb: bool,
    pub layers: Vec<Layer>,
    /// Document-level extra channels (saved selections / spot channels), which
    /// live after the color channels in the PSD image-data section.
    pub channels: Vec<Channel>,
    /// Verbatim color-mode-data section bytes, preserved for lossless re-save.
    pub color_mode_data: Vec<u8>,
    /// Verbatim image-resource section bytes, preserved for lossless re-save.
    pub image_resources: Vec<u8>,
    /// Global layer mask payload, preserved verbatim for lossless re-save.
    pub global_layer_mask: Vec<u8>,
    /// Bytes after the global layer mask up to the layer-section end, preserved
    /// verbatim for lossless re-save.
    pub layer_section_extra: Vec<u8>,
    /// User slices drawn with the Slice tool, in document pixels; auto slices
    /// are derived from them. ponytail: not yet written to or read from the
    /// slices image resource (1050), which stays preserved verbatim.
    pub slices: Vec<PsdRect>,
    /// Color samplers and notes; they ride the history snapshot like slices.
    pub annotations: Annotations,
    /// Guides in placement order; they ride the history snapshot. Read from
    /// and written to the PSD grid-and-guides resource (1032).
    pub guides: Vec<Guide>,
    /// The Work Path the Pen tool group draws; it rides the history snapshot.
    /// Read from and written to the PSD Work Path resource (1025).
    pub work_path: path::VectorPath,
    /// The saved paths the Paths panel lists above the Work Path, in panel
    /// order; they ride the history snapshot. Read from and written to the PSD
    /// saved-path resources (2000-2997).
    pub saved_paths: Vec<path::NamedPath>,
    /// Named character and paragraph styles, carried on the history snapshot so
    /// a style edit is undone with the document. `Basic Paragraph` is always
    /// present. ponytail: not yet written to or read from PSD (the style block's
    /// location is unresolved); an existing block is preserved verbatim.
    pub text_styles: TextStyleSheet,
}

impl Document {
    pub fn new(width: u32, height: u32, mode: ColorMode, depth: BitDepth) -> Self {
        let channels = mode.color_channels();
        Self {
            width,
            height,
            mode,
            depth,
            source_mode: None,
            source_depth: None,
            source_planes: None,
            source_palette: None,
            source_icc: None,
            document_icc: None,
            composite: PixelBuffer::new(width, height, channels),
            merged_composite_present: true,
            composite_compression: Compression::Rle,
            layer_compression: Compression::Rle,
            is_psb: false,
            layers: Vec::new(),
            channels: Vec::new(),
            color_mode_data: Vec::new(),
            image_resources: Vec::new(),
            global_layer_mask: Vec::new(),
            layer_section_extra: Vec::new(),
            slices: Vec::new(),
            annotations: Annotations::default(),
            guides: Vec::new(),
            work_path: path::VectorPath::default(),
            saved_paths: Vec::new(),
            text_styles: TextStyleSheet::default(),
        }
    }

    /// Build an RGB/8-bit document from packed RGBA8888 bytes (one pixel per
    /// four bytes, byte order R, G, B, A).
    ///
    /// The composite is a 4-plane RGBA [`PixelBuffer`] and the document holds
    /// exactly one pixel layer named `name`, both seeded from `rgba`. A buffer
    /// shorter than `width * height * 4` leaves the missing pixels transparent
    /// instead of panicking.
    pub fn from_rgba(name: &str, width: u32, height: u32, rgba: &[u8]) -> Document {
        let plane = width as usize * height as usize;
        let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
        let mut composite = vec![0u8; plane * 4];
        let mut channels = vec![
            Channel {
                id: 0,
                data: vec![0; plane].into(),
            },
            Channel {
                id: 1,
                data: vec![0; plane].into(),
            },
            Channel {
                id: 2,
                data: vec![0; plane].into(),
            },
            Channel {
                id: -1,
                data: vec![0; plane].into(),
            },
        ];
        for i in 0..plane {
            let at = i * 4;
            if at + 4 > rgba.len() {
                break;
            }
            for (c, value) in [
                (0, rgba[at]),
                (1, rgba[at + 1]),
                (2, rgba[at + 2]),
                (3, rgba[at + 3]),
            ] {
                channels[c].data[i] = value;
                composite[c * plane + i] = value;
            }
        }
        doc.composite = PixelBuffer {
            width,
            height,
            channels: 4,
            data: composite.into(),
        };
        doc.layers.push(Layer {
            name: name.to_string(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: height as i32,
                right: width as i32,
            },
            channels,
            ..Default::default()
        });
        doc
    }

    /// True when a read retained native 16/32-bit samples for some plane, so
    /// `write_psd` re-emits the source depth. False for an 8-bit read (including
    /// an 8-bit Lab read, whose retained store is 8-bit) and a constructed
    /// document, which still save 8-bit.
    pub fn retains_source_depth(&self) -> bool {
        self.source_planes
            .as_ref()
            .is_some_and(|store| store.depth != BitDepth::Eight)
            || retains_source_depth(&self.layers)
    }
}

/// Whether any layer in a tree retained native 16/32-bit channel samples.
fn retains_source_depth(layers: &[Layer]) -> bool {
    layers.iter().any(|l| {
        l.source_channels
            .as_ref()
            .is_some_and(|store| store.depth != BitDepth::Eight)
            || retains_source_depth(&l.children)
    })
}

impl Default for Document {
    fn default() -> Self {
        Document::new(0, 0, ColorMode::Rgb, BitDepth::Eight)
    }
}

/// A layer blend mode. The 27 modes CS6 exposes for a layer, plus the
/// group-only `'pass'` (Pass Through) option. Pass Through is not a layer mode:
/// it is not part of [`BlendMode::LAYER_MODES`] and must only be used on groups.
/// See `MODEL.md` for the key table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Dissolve,
    Darken,
    Multiply,
    ColorBurn,
    LinearBurn,
    DarkerColor,
    Lighten,
    Screen,
    ColorDodge,
    LinearDodge,
    LighterColor,
    Overlay,
    SoftLight,
    HardLight,
    VividLight,
    LinearLight,
    PinLight,
    HardMix,
    Difference,
    Exclusion,
    Subtract,
    Divide,
    Hue,
    Saturation,
    Color,
    Luminosity,
    /// Group-only: children blend directly against the parent backdrop.
    PassThrough,
}

impl BlendMode {
    /// The 27 layer modes, in PSD-spec table order (same order as the key list
    /// below). [`BlendMode::PassThrough`] is deliberately excluded.
    pub const LAYER_MODES: [BlendMode; 27] = [
        BlendMode::Normal,
        BlendMode::Dissolve,
        BlendMode::Darken,
        BlendMode::Multiply,
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::DarkerColor,
        BlendMode::Lighten,
        BlendMode::Screen,
        BlendMode::ColorDodge,
        BlendMode::LinearDodge,
        BlendMode::LighterColor,
        BlendMode::Overlay,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::VividLight,
        BlendMode::LinearLight,
        BlendMode::PinLight,
        BlendMode::HardMix,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Subtract,
        BlendMode::Divide,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ];

    /// The 4-byte PSD blend-mode key.
    pub fn to_psd_key(&self) -> [u8; 4] {
        match self {
            BlendMode::Normal => *b"norm",
            BlendMode::Dissolve => *b"diss",
            BlendMode::Darken => *b"dark",
            BlendMode::Multiply => *b"mul ",
            BlendMode::ColorBurn => *b"idiv",
            BlendMode::LinearBurn => *b"lbrn",
            BlendMode::DarkerColor => *b"dkCl",
            BlendMode::Lighten => *b"lite",
            BlendMode::Screen => *b"scrn",
            BlendMode::ColorDodge => *b"div ",
            BlendMode::LinearDodge => *b"lddg",
            BlendMode::LighterColor => *b"lgCl",
            BlendMode::Overlay => *b"over",
            BlendMode::SoftLight => *b"sLit",
            BlendMode::HardLight => *b"hLit",
            BlendMode::VividLight => *b"vLit",
            BlendMode::LinearLight => *b"lLit",
            BlendMode::PinLight => *b"pLit",
            BlendMode::HardMix => *b"hMix",
            BlendMode::Difference => *b"diff",
            BlendMode::Exclusion => *b"smud",
            BlendMode::Subtract => *b"fsub",
            BlendMode::Divide => *b"fdiv",
            BlendMode::Hue => *b"hue ",
            BlendMode::Saturation => *b"sat ",
            BlendMode::Color => *b"colr",
            BlendMode::Luminosity => *b"lum ",
            BlendMode::PassThrough => *b"pass",
        }
    }

    /// Parse a 4-byte PSD key; `None` for unknown keys.
    pub fn from_psd_key(key: [u8; 4]) -> Option<BlendMode> {
        if key == *b"pass" {
            return Some(BlendMode::PassThrough);
        }
        Self::LAYER_MODES
            .into_iter()
            .find(|m| m.to_psd_key() == key)
    }
}

/// A signed rectangle in PSD coordinates. Bounds may fall outside the canvas,
/// so all edges are `i32` and `right`/`bottom` may be smaller than `left`/`top`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsdRect {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

impl PsdRect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// One planar channel of layer data.
///
/// `id`: `0,1,2…` color, `-1` transparency mask, `-2` user layer mask,
/// `-3` real user mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: i16,
    pub data: Plane<u8>,
}

/// The raw additional-layer-info block of an adjustment layer.
///
/// `key` is the 4-byte PSD adjustment key (e.g. `nvrt`, `brit`, `levl`) and
/// `data` is its payload, stored verbatim. `pictura-core` does not interpret
/// the payload; `pictura-render` decodes the subset it understands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustmentData {
    pub key: [u8; 4],
    pub data: Vec<u8>,
}

/// An additional-layer-info tagged block the engine does not model, preserved
/// verbatim for lossless re-save.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerBlock {
    pub key: [u8; 4],
    pub data: Vec<u8>,
}

/// A layer channel the engine does not decode, stored as its full on-disk
/// stream including the 2-byte compression header for verbatim re-emit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawChannel {
    pub id: i16,
    pub data: Vec<u8>,
}

/// Retained source-depth samples of a document's color planes and extra
/// channels, kept so an unchanged plane can be re-emitted exactly.
///
/// `samples` is the planar store: the composite color channels followed by the
/// document extra channels, each `width * height` native samples. Set for a
/// 16/32-bit Grayscale or RGB read, and for an 8-bit Lab read (whose `depth` is
/// `Eight` and whose samples are the Lab color planes).
#[derive(Debug, Clone, PartialEq)]
pub struct SourcePlanes {
    pub depth: BitDepth,
    pub width: u32,
    pub height: u32,
    pub samples: Samples,
}

/// A layer's retained native-depth channel samples, keyed by channel id and
/// carrying the layer rect they were decoded for. A saved layer whose rect
/// still matches can re-emit them; a moved layer falls back to widening.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceChannels {
    pub depth: BitDepth,
    pub rect: PsdRect,
    /// `(channel id, plane samples)` sorted by channel id. A native-depth store
    /// holds the color channels, `-1` transparency, `-2` mask, and unmodeled
    /// ids; an 8-bit Lab store holds only the color channels (`0..3`).
    pub planes: Vec<(i16, Samples)>,
}

impl SourceChannels {
    /// Build a store, sorting the planes by channel id so equality does not
    /// depend on whether they were captured in read or write emission order.
    pub fn new(depth: BitDepth, rect: PsdRect, mut planes: Vec<(i16, Samples)>) -> Self {
        planes.sort_by_key(|(id, _)| *id);
        Self {
            depth,
            rect,
            planes,
        }
    }
}

/// How a smart object's source is linked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SmartObjectKind {
    /// `liFD`: the source bytes are embedded in the document.
    Embedded,
    /// `liFE`: the source is an external path; never read from disk.
    External,
    /// `liFA`: an alias link.
    Alias,
    /// A config descriptor exists but no matching linked record was found (or
    /// its parse failed). The preserved bytes remain the source of truth.
    #[default]
    Unresolved,
}

/// A typed view of one smart filter in a layer's `filterFX` list.
///
/// `options` is the canonical re-serialization of the filter's `Fltr`
/// descriptor. The preserved `SoLd` descriptor bytes remain the source of truth
/// for re-emission, so this view is ignored while they exist.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SmartFilter {
    pub filter_id: i32,
    pub name: String,
    pub enabled: bool,
    pub options: Vec<u8>,
}

/// A typed view of a layer's embedded/linked smart object, derived on read.
///
/// The raw config descriptor and the document-level linked record are preserved
/// separately and remain the source of truth for re-emission; this view only
/// exposes what the engine can resolve.
#[derive(Debug, Clone, PartialEq)]
pub struct SmartObject {
    /// The `Idnt`/`PlLd` uuid that links the layer config to its record.
    pub uuid: String,
    /// Original filename, with any trailing NUL stripped.
    pub filename: String,
    /// 4-byte file-type code stored in the record (e.g. `8BPB`).
    pub filetype: [u8; 4],
    /// 4-byte creator code stored in the record (e.g. `8BIM`).
    pub creator: [u8; 4],
    pub kind: SmartObjectKind,
    /// Raw `SoLd`/`SoLE`/`plLd`/`PlLd` block data, kept verbatim.
    pub config_descriptor: Vec<u8>,
    /// Embedded source bytes; `Some` only for [`SmartObjectKind::Embedded`].
    pub payload: Option<Vec<u8>>,
    /// The `crs:` XMP packet found in an embedded payload, if any.
    pub crs_xmp: Option<Vec<u8>>,
    /// Typed view of the fixed `crs:` property set in [`SmartObject::crs_xmp`].
    pub crs: Option<CrsSettings>,
    /// Smart filters derived from the descriptor's `filterFX` list. The raw
    /// `config_descriptor` bytes remain the source of truth for re-emission.
    pub smart_filters: Vec<SmartFilter>,
    /// The `filterFXStyle.filterMaskEnable` group flag.
    pub filter_mask_enabled: bool,
    /// The `filterFXStyle.filterMaskLinked` group flag.
    pub filter_mask_linked: bool,
    /// The `filterFXStyle.filterMaskExtendWithWhite` group flag.
    pub filter_mask_extend_with_white: bool,
    /// The `filterFXStyle.enab` group flag: whether the whole smart-filter
    /// stack is enabled. Defaults to `true`.
    pub smart_filters_enabled: bool,
    /// The smart-filter mask, session-only. Masks pixels are not decoded yet,
    /// so this stays `None`; the preserved bytes carry the mask on disk.
    pub filter_mask: Option<LayerMask>,
}

impl Default for SmartObject {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            filename: String::new(),
            filetype: [0; 4],
            creator: [0; 4],
            kind: SmartObjectKind::default(),
            config_descriptor: Vec::new(),
            payload: None,
            crs_xmp: None,
            crs: None,
            smart_filters: Vec::new(),
            // PSD defaults: mask enabled and extending white; not linked.
            filter_mask_enabled: true,
            filter_mask_linked: false,
            filter_mask_extend_with_white: true,
            smart_filters_enabled: true,
            filter_mask: None,
        }
    }
}

/// A raster layer mask. `data` is `None` until the channel image is decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerMask {
    pub rect: PsdRect,
    pub default_color: u8,
    pub disabled: bool,
    pub flags: u8,
    pub data: Option<Plane<u8>>,
    /// Mask block bytes after the fixed 18-byte header, preserved verbatim for
    /// lossless re-save.
    pub extra: Vec<u8>,
}

impl Default for LayerMask {
    fn default() -> Self {
        Self {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            default_color: 0,
            disabled: false,
            flags: 0,
            data: None,
            extra: Vec::new(),
        }
    }
}

/// PSD `lclr` sheet color. Values match psd-tools `SheetColorType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ColorLabel {
    None = 0,
    Red = 1,
    Orange = 2,
    Yellow = 3,
    Green = 4,
    Blue = 5,
    Violet = 6,
    Gray = 7,
}

impl ColorLabel {
    pub fn from_byte(v: u8) -> ColorLabel {
        match v {
            0 => ColorLabel::None,
            1 => ColorLabel::Red,
            2 => ColorLabel::Orange,
            3 => ColorLabel::Yellow,
            4 => ColorLabel::Green,
            5 => ColorLabel::Blue,
            6 => ColorLabel::Violet,
            7 => ColorLabel::Gray,
            _ => ColorLabel::None,
        }
    }

    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

/// The four CS6 layer locks. A `u8` bit set, not an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LockFlags(u8);

impl LockFlags {
    pub const TRANSPARENCY: u8 = 0x01;
    pub const PIXELS: u8 = 0x02;
    pub const POSITION: u8 = 0x04;
    pub const NESTING: u8 = 0x08;

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub fn contains(self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    pub fn with(self, flag: u8, on: bool) -> LockFlags {
        if on {
            LockFlags(self.0 | flag)
        } else {
            LockFlags(self.0 & !flag)
        }
    }

    /// All four lockable bits (the panel's "Lock All" toggle).
    pub fn all() -> LockFlags {
        LockFlags(0x01 | 0x02 | 0x04 | 0x08)
    }

    pub fn is_all(self) -> bool {
        self.0 & 0x0F == 0x0F
    }
}

/// True when a layer's `POSITION` lock forbids moving it.
///
/// Lives in `pictura-core` (the crate every mutation entry shares) so
/// `pictura-paint` need not depend on `pictura-render`.
pub fn layer_move_locked(layer: &Layer) -> bool {
    layer.lock.contains(LockFlags::POSITION)
}

/// True when a layer's `PIXELS` lock forbids mutating its pixels.
pub fn layer_pixel_locked(layer: &Layer) -> bool {
    layer.lock.contains(LockFlags::PIXELS)
}

/// True when a layer's `TRANSPARENCY` lock forbids an alpha-changing edit.
pub fn layer_transparency_locked(layer: &Layer) -> bool {
    layer.lock.contains(LockFlags::TRANSPARENCY)
}

/// A pixel layer or a group (`is_group`). Groups carry `children`, bottom-first
/// like everything else. `rect` is the layer bounds; for groups it may be empty.
///
/// An adjustment layer carries `adjustment` and, in PSD, no color channels (its
/// mask still uses channel `-2`). `adjustment` is opaque to this crate.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub rect: PsdRect,
    pub blend: BlendMode,
    pub opacity: u8,
    pub fill: u8,
    pub lock: LockFlags,
    pub color: ColorLabel,
    pub clipping: bool,
    pub visible: bool,
    pub mask: Option<LayerMask>,
    pub adjustment: Option<AdjustmentData>,
    pub channels: Vec<Channel>,
    pub children: Vec<Layer>,
    pub is_group: bool,
    /// First-class Background flag. PSD has no background bit; the codec infers
    /// it from the bottom top-level non-group named `"Background"` and writes
    /// that name back for a flagged layer (design D5).
    pub background: bool,
    /// Raw blend-mode key, stored only when it does not map to a known
    /// [`BlendMode`], preserved for lossless re-save.
    pub blend_key: Option<[u8; 4]>,
    /// Raw blending-ranges block bytes, preserved verbatim for lossless re-save.
    pub blending_ranges: Vec<u8>,
    /// `knko` knockout mode (`None` when the block is absent).
    pub knockout: Knockout,
    /// `clbl` blend clipped elements (CS6 default `true` when absent).
    pub blend_clipping: bool,
    /// `infx` blend interior elements (CS6 default `true` when absent).
    pub blend_interior: bool,
    /// Typed Blend If view of `blending_ranges`; `None` when empty or malformed.
    pub blend_if: Option<BlendIf>,
    /// Additional-layer-info tagged blocks the engine does not model, preserved
    /// verbatim for lossless re-save.
    pub extra_blocks: Vec<LayerBlock>,
    /// Layer channels the engine does not decode, kept as full on-disk streams
    /// for verbatim re-emit.
    pub raw_channels: Vec<RawChannel>,
    /// Derived embedded/linked smart-object view; `None` for ordinary layers.
    pub smart_object: Option<SmartObject>,
    /// Derived `vmsk` vector-mask view; `None` when absent or unparseable. The
    /// raw block remains in `extra_blocks` and is the serialization source.
    pub vector_mask: Option<VectorMask>,
    /// Derived `TySh` type-tool view; `None` when absent or unparseable. The
    /// raw block remains in `extra_blocks` and is the serialization source.
    pub type_tool: Option<TypeTool>,
    /// The named styles a type layer applies and the run's manual overrides, in
    /// memory only (a style definition is not written to PSD). Carried on the
    /// layer so a re-author within a session keeps them.
    pub applied_character_style: Option<String>,
    pub applied_paragraph_style: Option<String>,
    pub type_overrides: StyleOverrides,
    /// Retained source channel samples, re-emitted on save when the layer has
    /// not moved: native `16`/`32`-bit samples for a Grayscale/RGB read, or the
    /// 8-bit Lab color planes for a Lab read.
    pub source_channels: Option<SourceChannels>,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            name: String::new(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask: None,
            adjustment: None,
            channels: Vec::new(),
            children: Vec::new(),
            is_group: false,
            background: false,
            blend_key: None,
            blending_ranges: Vec::new(),
            knockout: Knockout::None,
            blend_clipping: true,
            blend_interior: true,
            blend_if: None,
            extra_blocks: Vec::new(),
            raw_channels: Vec::new(),
            smart_object: None,
            vector_mask: None,
            type_tool: None,
            applied_character_style: None,
            applied_paragraph_style: None,
            type_overrides: StyleOverrides::default(),
            source_channels: None,
        }
    }
}

impl Layer {
    pub fn is_group(&self) -> bool {
        self.is_group
    }

    /// The preserved additional-layer-information block keyed `key`, if any.
    pub fn extra_block(&self, key: &[u8; 4]) -> Option<&LayerBlock> {
        self.extra_blocks.iter().find(|b| &b.key == key)
    }

    /// A type layer: any preserved `TySh` (type tool) block is present.
    pub fn is_type(&self) -> bool {
        self.extra_block(b"TySh").is_some()
    }
}

#[cfg(test)]
mod tests;
