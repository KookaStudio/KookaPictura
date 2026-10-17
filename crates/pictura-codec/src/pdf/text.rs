//! Type layers as live text (design D13): each one's glyphs set in its embedded
//! font over the page image, so a PDF reader can select, search, and copy the
//! words and a PDF editor can retype them. The engine (`pictura_render`) hands
//! the glyphs over shaped and laid out; this module embeds the faces and writes
//! the text operators.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use pictura_core::PixelBuffer;

use super::flate;
use super::object::{real, Body, Dict, Obj, Objects};

/// A face live text is set in, embedded whole: an editor can then set
/// characters the document never used, which a subset could not.
#[derive(Debug, Clone)]
pub struct PdfFont<'a> {
    /// The sfnt: TrueType, or CFF-flavoured OpenType when `cff`.
    pub program: &'a [u8],
    pub cff: bool,
    pub postscript: String,
    pub units_per_em: u16,
    /// xMin, yMin, xMax, yMax in font units.
    pub bbox: [i16; 4],
    pub ascent: i16,
    pub descent: i16,
    pub cap_height: i16,
    pub italic_angle: f32,
    pub fixed_pitch: bool,
    pub italic: bool,
    /// The advance of every glyph drawn in this face, in font units.
    pub widths: Vec<(u16, u16)>,
    /// What each drawn glyph reads as (its characters; a ligature's several).
    pub unicode: Vec<(u16, String)>,
}

/// One glyph: its id and its pen (the baseline start) in the layer's
/// untransformed frame, in document pixels relative to the text origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfGlyph {
    pub id: u16,
    pub u: f64,
    pub v: f64,
}

/// One type layer written as live text.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfText {
    /// Index into [`LiveText::fonts`].
    pub font: usize,
    /// The em size in document pixels.
    pub size: f64,
    /// The text origin in document pixels.
    pub origin: (f64, f64),
    /// The y-down linear part (`xx, xy, yx, yy`) mapping the untransformed
    /// frame onto the document about the origin.
    pub matrix: [f64; 4],
    /// Horizontal and vertical character scale (1.0 = 100%); the pens already
    /// carry the horizontal one.
    pub scale: [f64; 2],
    /// The fill in the document's working colour numbers (RGB; a gray document
    /// repeats its gray).
    pub color: [u8; 3],
    /// Layer opacity times fill opacity, 0–255.
    pub opacity: u8,
    pub glyphs: Vec<PdfGlyph>,
}

/// The type layers drawn as live text, bottom first, and the composite of the
/// document without them, which becomes the page image.
#[derive(Debug, Clone)]
pub struct LiveText<'a> {
    pub background: PixelBuffer,
    pub fonts: Vec<PdfFont<'a>>,
    pub texts: Vec<PdfText>,
}

/// What the options let live text use; a type layer needing more stays in the
/// page image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPolicy {
    /// Constant alpha for a faded layer (the soft-mask rule).
    pub transparency: bool,
    /// CFF outlines (`FontFile3 /OpenType`, PDF 1.6).
    pub cff: bool,
}

/// A text fill in the page image's colour space.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Fill {
    Gray(f64),
    Rgb([f64; 3]),
    Cmyk([f64; 4]),
    /// Components in the image's own colour space, named `/CS0` on the page.
    Space(Vec<f64>),
}

impl Fill {
    fn operator(&self) -> String {
        let join = |v: &[f64]| v.iter().map(|&c| real(c)).collect::<Vec<_>>().join(" ");
        match self {
            Fill::Gray(g) => format!("{} g", real(*g)),
            Fill::Rgb(c) => format!("{} rg", join(c)),
            Fill::Cmyk(c) => format!("{} k", join(c)),
            Fill::Space(c) => format!("/CS0 cs {} scn", join(c)),
        }
    }
}

/// An embedded font: its `Type0` object and the widths its `W` array states,
/// in thousandths of an em, which the `TJ` spacing is measured against.
pub(crate) struct Embedded {
    pub font: u32,
    widths: BTreeMap<u16, i64>,
}

/// Embed `font` as a `Type0` / `Identity-H` font with its descendant CIDFont,
/// descriptor, font program, and `ToUnicode` map; every object goes on `owned`.
pub(crate) fn embed_font(objects: &mut Objects, owned: &mut Vec<u32>, font: &PdfFont) -> Embedded {
    let upem = f64::from(font.units_per_em.max(1));
    let em = |v: f64| (v * 1000.0 / upem).round() as i64;
    let file_key = if font.cff { "FontFile3" } else { "FontFile2" };
    let mut file_dict = Dict::new().with("Filter", Obj::name("FlateDecode"));
    if font.cff {
        file_dict.set("Subtype", Obj::name("OpenType"));
    } else {
        file_dict.set("Length1", Obj::Int(font.program.len() as i64));
    }
    let file = objects.add(Body::Stream(file_dict, flate(font.program)));
    // Nonsymbolic (32), plus fixed pitch (1) and italic (64).
    let flags = 32 | i64::from(font.fixed_pitch) | (i64::from(font.italic) << 6);
    let descriptor = objects.add(Body::Obj(
        Dict::typed("FontDescriptor")
            .with("FontName", Obj::name(&font.postscript))
            .with("Flags", Obj::Int(flags))
            .with(
                "FontBBox",
                Obj::Array(
                    font.bbox
                        .iter()
                        .map(|&v| Obj::Int(em(f64::from(v))))
                        .collect(),
                ),
            )
            .with("ItalicAngle", Obj::Real(f64::from(font.italic_angle)))
            .with("Ascent", Obj::Int(em(f64::from(font.ascent))))
            .with("Descent", Obj::Int(em(f64::from(font.descent))))
            .with("CapHeight", Obj::Int(em(f64::from(font.cap_height))))
            // ponytail: no stem width is measured; 80 is the customary guess.
            .with("StemV", Obj::Int(80))
            .with(file_key, Obj::Ref(file))
            .into(),
    ));
    let widths: BTreeMap<u16, i64> = font
        .widths
        .iter()
        .map(|&(id, advance)| (id, em(f64::from(advance))))
        .collect();
    let w = widths
        .iter()
        .flat_map(|(&id, &width)| [Obj::Int(i64::from(id)), Obj::Array(vec![Obj::Int(width)])])
        .collect();
    let mut cid_dict = Dict::typed("Font")
        .with(
            "Subtype",
            Obj::name(if font.cff {
                "CIDFontType0"
            } else {
                "CIDFontType2"
            }),
        )
        .with("BaseFont", Obj::name(&font.postscript))
        .with(
            "CIDSystemInfo",
            Dict::new()
                .with("Registry", Obj::Str(b"Adobe".to_vec()))
                .with("Ordering", Obj::Str(b"Identity".to_vec()))
                .with("Supplement", Obj::Int(0))
                .into(),
        )
        .with("FontDescriptor", Obj::Ref(descriptor))
        .with("W", Obj::Array(w));
    if !font.cff {
        cid_dict.set("CIDToGIDMap", Obj::name("Identity"));
    }
    let cid = objects.add(Body::Obj(cid_dict.into()));
    let to_unicode = objects.add(Body::Stream(
        Dict::new().with("Filter", Obj::name("FlateDecode")),
        flate(to_unicode_cmap(&font.unicode).as_bytes()),
    ));
    let base = if font.cff {
        format!("{}-Identity-H", font.postscript)
    } else {
        font.postscript.clone()
    };
    let type0 = objects.add(Body::Obj(
        Dict::typed("Font")
            .with("Subtype", Obj::name("Type0"))
            .with("BaseFont", Obj::name(&base))
            .with("Encoding", Obj::name("Identity-H"))
            .with("DescendantFonts", Obj::Array(vec![Obj::Ref(cid)]))
            .with("ToUnicode", Obj::Ref(to_unicode))
            .into(),
    ));
    owned.extend([file, descriptor, cid, to_unicode, type0]);
    Embedded {
        font: type0,
        widths,
    }
}

/// The `ToUnicode` CMap: two-byte glyph ids to UTF-16BE, 100 per block.
fn to_unicode_cmap(unicode: &[(u16, String)]) -> String {
    let map: BTreeMap<u16, &str> = unicode
        .iter()
        .filter(|(_, text)| !text.is_empty())
        .map(|(id, text)| (*id, text.as_str()))
        .collect();
    let mut out = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let entries: Vec<_> = map.into_iter().collect();
    for block in entries.chunks(100) {
        let _ = writeln!(out, "{} beginbfchar", block.len());
        for (id, text) in block {
            let utf16: String = text.encode_utf16().map(|u| format!("{u:04X}")).collect();
            let _ = writeln!(out, "<{id:04X}> <{utf16}>");
        }
        out.push_str("endbfchar\n");
    }
    out.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    out
}

/// The page geometry text is drawn into: `points` per document pixel and the
/// page height in points (PDF's y runs up).
pub(crate) struct Page {
    pub points: f64,
    pub height: f64,
}

/// The content-stream operators drawing `text` in font resource `font_name`
/// (`/F<n>`), filled with `fill`, under graphics state `alpha` (`/GS<n>`) when
/// it is faded. Glyphs sharing a baseline go in one `TJ`; a new baseline (a
/// new line, a mark, a vertical column's next cell) sets a new text matrix.
pub(crate) fn operators(
    text: &PdfText,
    font_name: &str,
    font: &Embedded,
    fill: &Fill,
    alpha: Option<&str>,
    page: &Page,
) -> String {
    let mut out = String::from("q\n");
    if let Some(gs) = alpha {
        let _ = writeln!(out, "/{gs} gs");
    }
    let _ = writeln!(out, "{}", fill.operator());
    let _ = writeln!(out, "BT\n/{font_name} {} Tf", real(text.size));
    let h = if text.scale[0] > 0.0 {
        text.scale[0]
    } else {
        1.0
    };
    let v = text.scale[1];
    let [xx, xy, yx, yy] = text.matrix;
    let s = page.points;
    let size = if text.size > 0.0 { text.size } else { 1.0 };
    let mut runs: Vec<Vec<PdfGlyph>> = Vec::new();
    for &glyph in &text.glyphs {
        match runs.last_mut() {
            Some(run)
                if (glyph.v - run[0].v).abs() < 1e-6
                    && glyph.u >= run.last().map_or(f64::MIN, |g| g.u) =>
            {
                run.push(glyph);
            }
            _ => runs.push(vec![glyph]),
        }
    }
    for run in runs {
        let (u0, v0) = (run[0].u, run[0].v);
        let (ox, oy) = text.origin;
        let tm = [
            s * xx * h,
            -s * xy * h,
            -s * yx * v,
            s * yy * v,
            s * (ox + xx * u0 + yx * v0),
            page.height - s * (oy + xy * u0 + yy * v0),
        ];
        let tm: Vec<String> = tm.iter().map(|&c| real(c)).collect();
        let _ = write!(out, "{} Tm [", tm.join(" "));
        for (i, glyph) in run.iter().enumerate() {
            if i > 0 {
                let previous = run[i - 1];
                let stated = font.widths.get(&previous.id).copied().unwrap_or(1000) as f64;
                let actual = (glyph.u - previous.u) / h * 1000.0 / size;
                let adjust = stated - actual;
                if adjust.abs() >= 0.01 {
                    let _ = write!(out, " {} ", real(adjust));
                }
            }
            let _ = write!(out, "<{:04X}>", glyph.id);
        }
        out.push_str("] TJ\n");
    }
    out.push_str("ET\nQ\n");
    out
}
