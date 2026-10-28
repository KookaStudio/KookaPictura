//! Fill-content bridge: read and edit a Solid Color, Gradient, or Pattern fill
//! layer, list the document's patterns, and create a fill layer with the
//! editor's chosen content. Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow). Each write
//! recomposites and records one state.

use super::super::edit::Edit;
use super::super::helpers_composite::{default_layer_mask, rgba_image, selection_to_mask};
use super::super::impl_layers_create::{decode_blend_key, decode_color};
use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString, QStringList};
use pictura_core::{BlendMode, Document, Layer, PsdRect};
use pictura_render::{
    Adjustment, GradientFillParams, GradientKind, GradientStop, PatternFillParams,
};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// `"solid-color"`, `"gradient"`, `"pattern"`, or empty for a non-fill.
        fn layer_fill_kind(view: &PictureView, path: &QString) -> QString;
        /// A Solid Color fill's packed ARGB, or 0.
        fn layer_fill_color(view: &PictureView, path: &QString) -> u32;
        /// `kind|angle|scale|reverse|loc:rrggbb,…`; empty for a non-gradient.
        fn layer_fill_gradient(view: &PictureView, path: &QString) -> QString;
        /// `id|scale|link|x|y`; empty for a non-pattern.
        fn layer_fill_pattern(view: &PictureView, path: &QString) -> QString;
        fn set_layer_fill_color(view: Pin<&mut PictureView>, path: &QString, color: u32) -> bool;
        fn set_layer_fill_gradient(
            view: Pin<&mut PictureView>,
            path: &QString,
            payload: &QString,
        ) -> bool;
        fn set_layer_fill_pattern(
            view: Pin<&mut PictureView>,
            path: &QString,
            payload: &QString,
        ) -> bool;
        /// Whether the document carries any `Patt`/`Pat2`/`Pat3` pattern.
        fn layer_has_patterns(view: &PictureView) -> bool;
        /// Whether the layer is a fill content layer the fill editors can edit
        /// (a shape's vector fill is excluded).
        fn layer_is_editable_fill(view: &PictureView, path: &QString) -> bool;
        /// The document's pattern ids in decode order.
        fn document_pattern_ids(view: &PictureView) -> QStringList;
        /// A `size`-bounded thumbnail for pattern `id`, or a null image.
        fn pattern_thumbnail(view: &PictureView, id: &QString, size: i32) -> QImage;
        /// Create a fill layer of `kind` from the editor payload, anchored at
        /// `selection_path` (empty = top). One history state.
        #[allow(clippy::too_many_arguments)]
        fn add_fill_layer(
            view: Pin<&mut PictureView>,
            kind: &QString,
            payload: &QString,
            selection_path: &QString,
            name: &QString,
            color: i32,
            blend_key: &QString,
            opacity: i32,
            clipping: bool,
        ) -> QString;
    }
}

fn layer<'a>(doc: &'a Document, path: &str) -> Option<&'a Layer> {
    pictura_render::resolve_path(doc, path)
}

fn fill_adjustment(doc: &Document, path: &str) -> Option<Adjustment> {
    let layer = layer(doc, path)?;
    if pictura_render::is_shape_layer(layer) {
        return None;
    }
    Some(layer)
        .and_then(pictura_render::decode_layer_fill)
        .filter(|adjustment| {
            matches!(
                adjustment,
                Adjustment::SolidFill(_) | Adjustment::GradientFill(_) | Adjustment::PatternFill(_)
            )
        })
}

fn layer_fill_kind(view: &PictureView, path: &QString) -> QString {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return QString::default();
    };
    match fill_adjustment(doc, &path.to_string()) {
        Some(Adjustment::SolidFill(_)) => QString::from("solid-color"),
        Some(Adjustment::GradientFill(_)) => QString::from("gradient"),
        Some(Adjustment::PatternFill(_)) => QString::from("pattern"),
        _ => QString::default(),
    }
}

fn layer_fill_color(view: &PictureView, path: &QString) -> u32 {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return 0;
    };
    match fill_adjustment(doc, &path.to_string()) {
        Some(Adjustment::SolidFill([r, g, b, a])) => {
            (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32
        }
        _ => 0,
    }
}

fn kind_key(kind: GradientKind) -> &'static str {
    match kind {
        GradientKind::Linear => "linear",
        GradientKind::Radial => "radial",
        GradientKind::Angle => "angled",
        GradientKind::Reflected => "reflected",
        GradientKind::Diamond => "diamond",
    }
}

fn kind_from_key(key: &str) -> Option<GradientKind> {
    Some(match key {
        "linear" => GradientKind::Linear,
        "radial" => GradientKind::Radial,
        "angled" => GradientKind::Angle,
        "reflected" => GradientKind::Reflected,
        "diamond" => GradientKind::Diamond,
        _ => return None,
    })
}

fn format_stops(stops: &[GradientStop]) -> String {
    stops
        .iter()
        .map(|stop| {
            format!(
                "{}:{:02x}{:02x}{:02x}",
                stop.location, stop.color[0], stop.color[1], stop.color[2]
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn layer_fill_gradient(view: &PictureView, path: &QString) -> QString {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return QString::default();
    };
    match fill_adjustment(doc, &path.to_string()) {
        Some(Adjustment::GradientFill(params)) => QString::from(
            format!(
                "{}|{}|{}|{}|{}",
                kind_key(params.kind),
                params.angle_deg,
                params.scale,
                u8::from(params.reverse),
                format_stops(&params.stops)
            )
            .as_str(),
        ),
        _ => QString::default(),
    }
}

fn layer_fill_pattern(view: &PictureView, path: &QString) -> QString {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return QString::default();
    };
    match fill_adjustment(doc, &path.to_string()) {
        Some(Adjustment::PatternFill(params)) => QString::from(
            format!(
                "{}|{}|{}|{}|{}",
                params.pattern_id,
                params.scale,
                u8::from(params.link_with_layer),
                params.origin.0,
                params.origin.1
            )
            .as_str(),
        ),
        _ => QString::default(),
    }
}

fn parse_gradient(payload: &str) -> Option<GradientFillParams> {
    let mut parts = payload.split('|');
    let kind = kind_from_key(parts.next()?)?;
    let angle_deg: f32 = parts.next()?.parse().ok()?;
    let scale: f32 = parts.next()?.parse().ok()?;
    let reverse = parts.next()? == "1";
    let stops = parts
        .next()?
        .split(',')
        .filter(|stop| !stop.is_empty())
        .map(|stop| {
            let (location, hex) = stop.split_once(':')?;
            let location: u16 = location.parse().ok()?;
            let value = u32::from_str_radix(hex.trim_start_matches('#'), 16).ok()?;
            Some(GradientStop {
                location,
                color: [(value >> 16) as u8, (value >> 8) as u8, value as u8],
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if stops.len() < 2 || !angle_deg.is_finite() || !scale.is_finite() {
        return None;
    }
    Some(GradientFillParams {
        stops,
        reverse,
        kind,
        angle_deg,
        scale,
    })
}

fn parse_pattern(payload: &str) -> Option<PatternFillParams> {
    let mut parts = payload.split('|');
    let pattern_id = parts.next()?.to_string();
    if pattern_id.is_empty() {
        return None;
    }
    let scale: f32 = parts.next()?.parse().ok()?;
    let link_with_layer = parts.next()? == "1";
    let x: i32 = parts.next()?.parse().ok()?;
    let y: i32 = parts.next()?.parse().ok()?;
    if !scale.is_finite() {
        return None;
    }
    Some(PatternFillParams {
        pattern_id,
        scale,
        link_with_layer,
        origin: (x, y),
    })
}

fn layer_rect(layer: &Layer) -> Option<PsdRect> {
    let rect = layer.rect;
    (rect.width() > 0 && rect.height() > 0).then_some(rect)
}

fn set_fill_adjustment(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    label: &str,
    data: pictura_core::AdjustmentData,
) -> bool {
    let path = path.to_string();
    let mut changed = false;
    view.as_mut().edit(label, |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Some(layer) = pictura_render::resolve_path_mut(doc, &path) else {
            return Edit::None;
        };
        if pictura_render::is_shape_layer(layer) || layer.adjustment.as_ref() == Some(&data) {
            return Edit::None;
        }
        layer.adjustment = Some(data);
        changed = true;
        match layer_rect(layer) {
            // The fill preview lives in the row thumbnail, so the panel
            // must refresh after an accepted edit.
            Some(rect) => Edit::regions(vec![rect], true),
            None => Edit::Full,
        }
    });
    changed
}

fn set_layer_fill_color(view: Pin<&mut PictureView>, path: &QString, color: u32) -> bool {
    let rgba = [
        (color >> 16) as u8,
        (color >> 8) as u8,
        color as u8,
        (color >> 24) as u8,
    ];
    set_fill_adjustment(
        view,
        path,
        "Solid Color Fill",
        pictura_render::encode_solid_color_fill([rgba[0], rgba[1], rgba[2]]),
    )
}

fn set_layer_fill_gradient(view: Pin<&mut PictureView>, path: &QString, payload: &QString) -> bool {
    let Some(params) = parse_gradient(&payload.to_string()) else {
        return false;
    };
    set_fill_adjustment(
        view,
        path,
        "Gradient Fill",
        pictura_render::encode_gradient_fill_params(&params),
    )
}

fn set_layer_fill_pattern(view: Pin<&mut PictureView>, path: &QString, payload: &QString) -> bool {
    let Some(params) = parse_pattern(&payload.to_string()) else {
        return false;
    };
    set_fill_adjustment(
        view,
        path,
        "Pattern Fill",
        pictura_render::encode_pattern_fill(&params),
    )
}

fn layer_is_editable_fill(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| layer(doc, &path.to_string()))
        .is_some_and(|layer| {
            pictura_render::is_fill_content_layer(layer) && !pictura_render::is_shape_layer(layer)
        })
}

fn layer_has_patterns(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| !pictura_codec::decode_patterns(doc).is_empty())
}

fn document_pattern_ids(view: &PictureView) -> QStringList {
    let mut ids = QStringList::default();
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return ids;
    };
    for pattern in pictura_codec::decode_patterns(doc) {
        ids.append(QString::from(pattern.pattern_id.as_str()));
    }
    ids
}

fn pattern_thumbnail(view: &PictureView, id: &QString, size: i32) -> QImage {
    let size = size.clamp(1, 256) as u32;
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return QImage::default();
    };
    let patterns = pictura_codec::decode_patterns(doc);
    let Some(pattern) = patterns.iter().find(|p| p.pattern_id == id.to_string()) else {
        return QImage::default();
    };
    let pixels = (pattern.width as usize).saturating_mul(pattern.height as usize);
    if pattern.width == 0 || pattern.height == 0 || pattern.rgba.len() < pixels.saturating_mul(4) {
        return QImage::default();
    }
    let (tw, th) = if pattern.width >= pattern.height {
        (
            size,
            ((pattern.height as u64 * size as u64) / pattern.width as u64).max(1) as u32,
        )
    } else {
        (
            ((pattern.width as u64 * size as u64) / pattern.height as u64).max(1) as u32,
            size,
        )
    };
    let mut rgba = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        let sy = ty * pattern.height / th;
        for tx in 0..tw {
            let sx = tx * pattern.width / tw;
            let src = ((sy * pattern.width + sx) * 4) as usize;
            let dst = ((ty * tw + tx) * 4) as usize;
            rgba[dst..dst + 4].copy_from_slice(&pattern.rgba[src..src + 4]);
        }
    }
    rgba_image(rgba, tw as i32, th as i32)
}

#[allow(clippy::too_many_arguments)]
fn fill_layer(
    doc: &Document,
    kind: &str,
    payload: &str,
    name: &str,
    color: i32,
    blend: BlendMode,
    opacity: u8,
    clipping: bool,
) -> Option<Layer> {
    if doc.width == 0 || doc.height == 0 {
        return None;
    }
    let (name, adjustment) = match kind {
        "solid-color" => {
            let value = u32::from_str_radix(payload.trim_start_matches('#'), 16).ok()?;
            (
                if name.is_empty() {
                    pictura_render::next_layer_name(doc, "Color Fill")
                } else {
                    name.to_string()
                },
                pictura_render::encode_solid_color_fill([
                    (value >> 16) as u8,
                    (value >> 8) as u8,
                    value as u8,
                ]),
            )
        }
        "gradient" => (
            if name.is_empty() {
                pictura_render::next_layer_name(doc, "Gradient Fill")
            } else {
                name.to_string()
            },
            pictura_render::encode_gradient_fill_params(&parse_gradient(payload)?),
        ),
        "pattern" => (
            if name.is_empty() {
                pictura_render::next_layer_name(doc, "Pattern Fill")
            } else {
                name.to_string()
            },
            pictura_render::encode_pattern_fill(&parse_pattern(payload)?),
        ),
        _ => return None,
    };
    Some(Layer {
        name,
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        },
        blend,
        opacity,
        fill: 255,
        lock: pictura_core::LockFlags::default(),
        color: decode_color(color),
        clipping,
        visible: true,
        mask: None,
        adjustment: Some(adjustment),
        channels: Vec::new(),
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    })
}

#[allow(clippy::too_many_arguments)]
fn add_fill_layer(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    payload: &QString,
    selection_path: &QString,
    name: &QString,
    color: i32,
    blend_key: &QString,
    opacity: i32,
    clipping: bool,
) -> QString {
    let kind = kind.to_string();
    let payload = payload.to_string();
    let selection_path = selection_path.to_string();
    let name = name.to_string();
    let blend = decode_blend_key(&blend_key.to_string()).unwrap_or(BlendMode::Normal);
    let opacity = opacity.clamp(0, 255) as u8;
    let use_mask = view.rust().use_default_masks;
    let mut created = String::new();
    view.as_mut().edit("New Fill Layer", |rust| {
        let DocumentSession { doc, selection, .. } = &mut rust.session;
        let Some(doc) = doc.as_mut() else {
            return Edit::None;
        };
        let layer = fill_layer(doc, &kind, &payload, &name, color, blend, opacity, clipping);
        let Some(mut layer) = layer else {
            return Edit::None;
        };
        if use_mask {
            layer.mask = Some(default_layer_mask(
                doc,
                selection
                    .as_ref()
                    .map(|selection| selection_to_mask(selection, doc)),
            ));
        }
        created = pictura_render::insert_layer(doc, &selection_path, layer);
        if created.is_empty() {
            return Edit::None;
        }
        rust.select_only_layer(&created);
        Edit::Structural { damage: None }
    });
    QString::from(created.as_str())
}

use super::super::document_session::DocumentSession;
