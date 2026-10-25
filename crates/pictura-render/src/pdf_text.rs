//! Photoshop PDF live text (`interop/pdf-export`, design D13): which type
//! layers the page can carry as text rather than pixels, their laid-out glyphs,
//! and the faces to embed. A type layer qualifies only when nothing composites
//! over it, so drawing it after the page image changes no pixel beneath it.

use pictura_codec::pdf::{PdfFont, PdfGlyph, PdfText, TextPolicy};
use pictura_core::{BlendMode, ColorMode, Document, Layer, TypeSpec};
use rustybuzz::ttf_parser;

use crate::text_render::BundledText;

/// The live text of a document and the document with those layers hidden, which
/// the caller composites into the page image.
pub struct LiveType {
    pub hidden: Document,
    pub fonts: Vec<PdfFont<'static>>,
    pub texts: Vec<PdfText>,
}

/// The type layers `doc` can write as live text under `policy`, bottom first;
/// `None` when there are none.
///
/// ponytail: a type layer under any other visible content stays in the page
/// image, as do non-Normal blends and layer effects; drawing those as text
/// would need the content above it split into transparent bands.
pub fn pdf_live_type(doc: &Document, policy: TextPolicy) -> Option<LiveType> {
    let mut hidden = doc.clone();
    let mut found = Found {
        doc,
        policy,
        faces: Vec::new(),
        fonts: Vec::new(),
        texts: Vec::new(),
    };
    found.walk(&mut hidden.layers);
    if found.texts.is_empty() {
        return None;
    }
    found.texts.reverse();
    Some(LiveType {
        hidden,
        fonts: found.fonts,
        texts: found.texts,
    })
}

/// The font family of every type layer, so the shell can register each face
/// before a save resolves it.
pub fn pdf_type_fonts(doc: &Document) -> Vec<String> {
    fn walk(layers: &[Layer], out: &mut Vec<String>) {
        for layer in layers {
            walk(&layer.children, out);
            let family = crate::type_layer_spec(layer).map(|s| s.character.font_family);
            if let Some(family) = family.filter(|f| !f.is_empty() && !out.contains(f)) {
                out.push(family);
            }
        }
    }
    let mut out = Vec::new();
    walk(&doc.layers, &mut out);
    out
}

struct Found<'d> {
    doc: &'d Document,
    policy: TextPolicy,
    faces: Vec<&'static BundledText>,
    fonts: Vec<PdfFont<'static>>,
    texts: Vec<PdfText>,
}

impl Found<'_> {
    /// Take qualifying type layers from the top of `layers` down, hiding each;
    /// false once something else covers what lies below.
    fn walk(&mut self, layers: &mut pictura_core::LayerStack) -> bool {
        for layer in layers.iter_mut().rev() {
            if !layer.visible || layer.opacity == 0 {
                continue;
            }
            if layer.is_group {
                if !neutral_group(layer) || !self.walk(&mut layer.children) {
                    return false;
                }
                continue;
            }
            match self.live_text(layer) {
                Some(text) => {
                    self.texts.push(text);
                    layer.visible = false;
                }
                None => return false,
            }
        }
        true
    }

    fn live_text(&mut self, layer: &Layer) -> Option<PdfText> {
        let tool = layer.type_tool.as_ref()?;
        if layer.blend != BlendMode::Normal
            || layer.clipping
            || layer.mask.is_some()
            || layer.vector_mask.is_some()
            || crate::composite::has_effect_block(layer)
            || !pictura_codec::type_is_plain(tool)
        {
            return None;
        }
        let opacity = ((u16::from(layer.opacity) * u16::from(layer.fill) + 127) / 255) as u8;
        if opacity < 255 && !self.policy.transparency {
            return None;
        }
        let spec = crate::type_layer_spec(layer)?;
        if crate::type_layer::type_needs_fallback(&spec) {
            return None;
        }
        let (face, pens) = crate::type_layer::type_pens(&spec)?;
        if pens.is_empty() || crate::type_style::needs_raster(face, &spec.character) {
            return None;
        }
        let font = self.font(face, &spec, &pens)?;
        let [r, g, b, _] = spec.rgba();
        let color = if self.doc.mode == ColorMode::Grayscale {
            let gray =
                (0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)).round() as u8;
            [gray; 3]
        } else {
            [r, g, b]
        };
        Some(PdfText {
            font,
            size: spec.character.size,
            origin: spec.origin,
            matrix: spec.matrix,
            scale: [
                spec.character.horizontal_scale / 100.0,
                spec.character.vertical_scale / 100.0,
            ],
            color,
            opacity,
            glyphs: pens
                .iter()
                .map(|&(id, u, v)| PdfGlyph { id, u, v })
                .collect(),
        })
    }

    /// The index of `face` among the embedded fonts, recording the widths and
    /// characters of `pens`; `None` when the face cannot be embedded.
    fn font(
        &mut self,
        face: &'static BundledText,
        spec: &TypeSpec,
        pens: &[(u16, f64, f64)],
    ) -> Option<usize> {
        let index = match self.faces.iter().position(|f| std::ptr::eq(*f, face)) {
            Some(index) => index,
            None => {
                let font = pdf_font(face).filter(|f| !f.cff || self.policy.cff)?;
                self.faces.push(face);
                self.fonts.push(font);
                self.faces.len() - 1
            }
        };
        let font = &mut self.fonts[index];
        let ttf = face.ttf();
        for &(id, _, _) in pens {
            if !font.widths.iter().any(|(g, _)| *g == id) {
                let advance = ttf.glyph_hor_advance(ttf_parser::GlyphId(id)).unwrap_or(0);
                font.widths.push((id, advance));
            }
        }
        for (id, text) in glyph_text(face, &spec.text) {
            if !font.unicode.iter().any(|(g, _)| *g == id) {
                font.unicode.push((id, text));
            }
        }
        Some(index)
    }
}

/// A group that composites its children as if they sat in its parent.
fn neutral_group(group: &Layer) -> bool {
    matches!(group.blend, BlendMode::PassThrough | BlendMode::Normal)
        && group.opacity == 255
        && group.fill == 255
        && !group.clipping
        && group.mask.is_none()
        && group.vector_mask.is_none()
        && group.extra_block(b"lfx2").is_none()
        && group.extra_block(b"lrFX").is_none()
}

/// `face` as an embeddable font, without glyphs yet; `None` when its licence
/// forbids embedding or it has neither TrueType nor CFF outlines.
fn pdf_font(face: &BundledText) -> Option<PdfFont<'static>> {
    let ttf = face.ttf();
    if ttf.permissions() == Some(ttf_parser::Permissions::Restricted) {
        return None;
    }
    let tables = ttf.tables();
    let cff = match (tables.glyf.is_some(), tables.cff.is_some()) {
        (true, _) => false,
        (false, true) => true,
        (false, false) => return None,
    };
    let bbox = ttf.global_bounding_box();
    let postscript: String = face
        .postscript_name()
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_graphic())
        .collect();
    Some(PdfFont {
        program: face.bytes(),
        cff,
        postscript: if postscript.is_empty() {
            "PicturaFont".into()
        } else {
            postscript
        },
        units_per_em: ttf.units_per_em(),
        bbox: [bbox.x_min, bbox.y_min, bbox.x_max, bbox.y_max],
        ascent: ttf.ascender(),
        descent: ttf.descender(),
        cap_height: ttf.capital_height().unwrap_or_else(|| ttf.ascender()),
        italic_angle: ttf.italic_angle(),
        fixed_pitch: ttf.is_monospaced(),
        italic: ttf.is_italic(),
        widths: Vec::new(),
        unicode: Vec::new(),
    })
}

/// What each glyph shaping `text` produces reads as: its cluster's characters
/// (a ligature gets all of them), then each character's own cmap glyph.
fn glyph_text(face: &BundledText, text: &str) -> Vec<(u16, String)> {
    let mut out: Vec<(u16, String)> = Vec::new();
    for line in text.split(['\r', '\n']).filter(|l| !l.is_empty()) {
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(line);
        buffer.set_direction(rustybuzz::Direction::LeftToRight);
        buffer.guess_segment_properties();
        let shaped = rustybuzz::shape(face.shaper(), &[], buffer);
        let infos = shaped.glyph_infos();
        for (i, info) in infos.iter().enumerate() {
            let start = info.cluster as usize;
            if i > 0 && infos[i - 1].cluster as usize == start {
                continue;
            }
            let end = infos[i + 1..]
                .iter()
                .map(|next| next.cluster as usize)
                .find(|&c| c > start)
                .unwrap_or(line.len());
            if let (Ok(id), Some(chars)) = (u16::try_from(info.glyph_id), line.get(start..end)) {
                out.push((id, chars.to_string()));
            }
        }
    }
    for c in text.chars().filter(|c| !c.is_control()) {
        if let Some(id) = face.ttf().glyph_index(c) {
            out.push((id.0, c.to_string()));
        }
    }
    out
}

#[cfg(test)]
mod tests;
