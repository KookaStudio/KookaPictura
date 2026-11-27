use super::super::{
    edit::Edit,
    helpers::layer_has_effects,
    helpers_composite::{buffer_to_image, layer_buffer},
    qobject::PictureView,
    state::PictureViewRust,
};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QImage;
use pictura_core::memory::{governor, Reservation};
use pictura_core::{
    BitDepth, Channel, ColorMode, Layer, LayerKind, LockFlags, PixelBuffer, PsdRect,
};
use pictura_ops::puppet_warp::{Mesh, Pin as MeshPin, MAX_PIXELS};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        type PuppetWarpSession;

        fn puppet_warp_pin_angle(session: &PuppetWarpSession, ordinal: u32) -> f64;

        fn puppet_warp_available(view: &PictureView) -> bool;
        fn puppet_warp_begin(view: Pin<&mut PictureView>) -> Box<PuppetWarpSession>;
        fn puppet_warp_size(session: &PuppetWarpSession) -> Vec<i32>;
        // Pins are back to front: rest xy, target xy, angle (NaN for Auto), ordinal.
        fn puppet_warp_configure(
            session: &mut PuppetWarpSession,
            pins: &[f64],
            mode: i32,
            density: i32,
            expansion: i32,
        ) -> i32;
        fn puppet_warp_bounds(session: &PuppetWarpSession) -> Vec<i32>;
        fn puppet_warp_triangles(session: &PuppetWarpSession) -> Vec<f64>;
        fn puppet_warp_hit(session: &PuppetWarpSession, x: f64, y: f64) -> Vec<f64>;
        fn puppet_warp_render(
            session: &PuppetWarpSession,
            x: f64,
            y: f64,
            scale: f64,
            w: i32,
            h: i32,
        ) -> QImage;
        fn puppet_warp_commit(view: Pin<&mut PictureView>, session: &PuppetWarpSession) -> bool;
    }
}

const PREVIEW_PIXELS: u64 = 1024 * 1024;
const PIN_FIELDS: usize = 6;

pub struct PuppetWarpSession {
    snapshot: Option<Snapshot>,
    mesh: Option<Mesh>,
}

#[derive(PartialEq, Eq)]
struct Binding {
    path: String,
    mask_default: Option<u8>,
    linked_mask_default: Option<u8>,
    rect: PsdRect,
    document: u64,
    revision: u64,
    selection_revision: u64,
}

struct Snapshot {
    owner: usize,
    binding: Binding,
    pixels: PixelBuffer,
    linked_mask: Option<PixelBuffer>,
    linked_mask_density: Option<u8>,
    _memory: Reservation<'static>,
}

fn dimensions(rect: PsdRect) -> Option<(u32, u32)> {
    let w = u32::try_from(rect.right.checked_sub(rect.left)?).ok()?;
    let h = u32::try_from(rect.bottom.checked_sub(rect.top)?).ok()?;
    (w > 0 && h > 0 && u64::from(w) * u64::from(h) <= MAX_PIXELS as u64).then_some((w, h))
}

fn target(rust: &PictureViewRust) -> Option<(Binding, &Layer)> {
    if rust.session.read_only()
        || rust.session.quick_mask.active
        || rust.session.selection.is_some()
        || rust.session.pending_previews.any()
        || rust.has_active_session()
    {
        return None;
    }
    let doc = rust.session.doc.as_ref()?;
    if doc.source.depth().unwrap_or(doc.depth) != BitDepth::Eight
        || !matches!(doc.mode, ColorMode::Rgb | ColorMode::Grayscale)
        || !matches!(
            doc.source.mode().unwrap_or(doc.mode),
            ColorMode::Rgb | ColorMode::Grayscale
        )
    {
        return None;
    }
    let path = rust.layer_target.single()?;
    let layer = pictura_render::resolve_path(doc, path)?;
    if layer.kind() != LayerKind::Pixel
        || layer.background
        || !pictura_render::is_visible_in_panel(doc, path)
        || layer.pixel_data_irrelevant
        || layer.lock.contains(LockFlags::PIXELS | LockFlags::POSITION)
    {
        return None;
    }
    if layer
        .source_channels
        .as_ref()
        .is_some_and(|store| store.depth != BitDepth::Eight)
    {
        return None;
    }
    let mask_target = rust.mask_edit_target.as_deref() == Some(path);
    let (mut rect, mask_default) = if mask_target {
        let mask = layer.mask.as_ref()?;
        if layer.raw_channels().iter().any(|channel| channel.id == -3)
            || layer.channels().iter().any(|channel| channel.id == -3)
            || layer
                .source_channels
                .as_ref()
                .is_some_and(|store| store.planes.iter().any(|(id, _)| *id == -3))
        {
            return None;
        }
        let (w, h) = dimensions(mask.rect)?;
        if mask.data.as_ref()?.len() != w as usize * h as usize {
            return None;
        }
        (mask.rect, Some(mask.default_color))
    } else {
        if layer.lock.contains(LockFlags::TRANSPARENCY)
            || rust.session.link_sets.contains_key(path)
            || layer.vector_mask.is_some()
        {
            return None;
        }
        let (w, h) = dimensions(layer.rect)?;
        let n = w as usize * h as usize;
        let colors = if doc.mode == ColorMode::Rgb { 3 } else { 1 };
        let valid_id = |id| id == -2 || (-1..colors).contains(&id);
        if layer.raw_channels().iter().any(|c| !valid_id(c.id))
            || layer.channels().iter().any(|c| !valid_id(c.id))
            || (0..colors).any(|id| {
                layer
                    .channels()
                    .iter()
                    .filter(|c| c.id == id && c.data.len() == n)
                    .count()
                    != 1
            })
            || layer.channels().iter().filter(|c| c.id == -1).count() > 1
            || layer
                .channels()
                .iter()
                .any(|c| c.id == -1 && c.data.len() != n)
        {
            return None;
        }
        (layer.rect, None)
    };
    let linked_mask_default = if !mask_target && pictura_render::layer_mask_linked(doc, path) {
        let mask = layer.mask.as_ref()?;
        if layer.raw_channels().iter().any(|channel| channel.id == -3)
            || layer.channels().iter().any(|channel| channel.id == -3)
            || layer
                .source_channels
                .as_ref()
                .is_some_and(|store| store.planes.iter().any(|(id, _)| *id == -3))
        {
            return None;
        }
        let (w, h) = dimensions(mask.rect)?;
        if mask.data.as_ref()?.len() != w as usize * h as usize {
            return None;
        }
        rect = PsdRect {
            left: rect.left.min(mask.rect.left),
            top: rect.top.min(mask.rect.top),
            right: rect.right.max(mask.rect.right),
            bottom: rect.bottom.max(mask.rect.bottom),
        };
        dimensions(rect)?;
        Some(mask.default_color)
    } else {
        None
    };
    Some((
        Binding {
            path: path.to_owned(),
            mask_default,
            linked_mask_default,
            rect,
            document: rust.session.document_generation,
            revision: rust.session.content_revision,
            selection_revision: rust.session.selection_revision,
        },
        layer,
    ))
}

pub fn puppet_warp_available(view: &PictureView) -> bool {
    target(view.rust()).is_some()
}

fn pad_to_rect(
    source: &PixelBuffer,
    source_rect: PsdRect,
    rect: PsdRect,
    fill: u8,
) -> Option<PixelBuffer> {
    let (width, height) = dimensions(rect)?;
    let x = usize::try_from(source_rect.left.checked_sub(rect.left)?).ok()?;
    let y = usize::try_from(source_rect.top.checked_sub(rect.top)?).ok()?;
    let source_width = source.width as usize;
    let source_height = source.height as usize;
    if x.checked_add(source_width)? > width as usize
        || y.checked_add(source_height)? > height as usize
    {
        return None;
    }
    let mut padded = PixelBuffer::new(width, height, source.channels);
    padded.data.fill(fill);
    let source_plane = source.pixel_count();
    let dest_plane = width as usize * height as usize;
    for channel in 0..source.channels as usize {
        for row in 0..source_height {
            let from = channel * source_plane + row * source_width;
            let to = channel * dest_plane + (y + row) * width as usize + x;
            padded.data[to..to + source_width]
                .copy_from_slice(&source.data[from..from + source_width]);
        }
    }
    Some(padded)
}

fn source_pixels(binding: &Binding, layer: &Layer) -> Option<(PixelBuffer, Option<PixelBuffer>)> {
    let (width, height) = dimensions(binding.rect)?;
    let pixels = if binding.mask_default.is_some() {
        PixelBuffer {
            width,
            height,
            channels: 1,
            data: layer.mask.as_ref()?.data.clone()?,
        }
    } else {
        pad_to_rect(&layer_buffer(layer)?, layer.rect, binding.rect, 0)?
    };
    let linked_mask = if let Some(default) = binding.linked_mask_default {
        let mask = layer.mask.as_ref()?;
        let (mask_width, mask_height) = dimensions(mask.rect)?;
        let source = PixelBuffer {
            width: mask_width,
            height: mask_height,
            channels: 1,
            data: mask.data.clone()?,
        };
        Some(pad_to_rect(&source, mask.rect, binding.rect, default)?)
    } else {
        None
    };
    Some((pixels, linked_mask))
}

fn snapshot(view: &PictureView) -> Option<Snapshot> {
    let (binding, layer) = target(view.rust())?;
    let (width, height) = dimensions(binding.rect)?;
    let bytes = u64::from(width) * u64::from(height) * 10;
    let memory = governor().reserve(bytes, "Puppet Warp source").ok()?;
    let (pixels, linked_mask) = source_pixels(&binding, layer)?;
    let linked_mask_density = binding.linked_mask_default.and_then(|_| {
        layer
            .mask
            .as_ref()
            .filter(|mask| !mask.disabled)
            .map(|mask| mask.density)
    });
    Some(Snapshot {
        owner: std::ptr::from_ref(view).addr(),
        binding,
        pixels,
        linked_mask,
        linked_mask_density,
        _memory: memory,
    })
}

pub fn puppet_warp_begin(mut view: Pin<&mut PictureView>) -> Box<PuppetWarpSession> {
    view.as_mut().settle_pending_composite();
    Box::new(PuppetWarpSession {
        snapshot: snapshot(&view),
        mesh: None,
    })
}

pub fn puppet_warp_size(session: &PuppetWarpSession) -> Vec<i32> {
    session.snapshot.as_ref().map_or_else(Vec::new, |snap| {
        vec![snap.pixels.width as i32, snap.pixels.height as i32]
    })
}

fn parse_pin(pin: &[f64]) -> Option<MeshPin> {
    let ordinal = pin[5];
    if !(0.0..=f64::from(u32::MAX)).contains(&ordinal) || ordinal.fract() != 0.0 {
        return None;
    }
    Some(MeshPin {
        rest: [pin[0], pin[1]],
        target: [pin[2], pin[3]],
        angle: (!pin[4].is_nan()).then_some(pin[4]),
        ordinal: ordinal as u32,
    })
}

fn solved(
    session: &PuppetWarpSession,
    pins: &[f64],
    mode: i32,
    density: i32,
    expansion: i32,
) -> Option<Mesh> {
    if pins.len() > 32 * PIN_FIELDS || !pins.len().is_multiple_of(PIN_FIELDS) {
        return None;
    }
    let snap = session.snapshot.as_ref()?;
    let pins = pins
        .chunks_exact(PIN_FIELDS)
        .map(parse_pin)
        .collect::<Option<Vec<_>>>()?;
    let mesh = Mesh::solve(
        snap.pixels.width,
        snap.pixels.height,
        &pins,
        mode,
        density,
        expansion,
    )?;
    let bounds = mesh.bounds();
    let (w, h) = dimensions(bounds)?;
    let origin = [f64::from(bounds.left), f64::from(bounds.top)];
    if !mesh.can_raster(origin, 1.0, w, h) || offset(bounds, snap.binding.rect).is_none() {
        return None;
    }
    Some(mesh)
}

pub fn puppet_warp_configure(
    session: &mut PuppetWarpSession,
    pins: &[f64],
    mode: i32,
    density: i32,
    expansion: i32,
) -> i32 {
    session.mesh = solved(session, pins, mode, density, expansion);
    session
        .mesh
        .as_ref()
        .map_or(0, |mesh| if mesh.is_identity() { 1 } else { 2 })
}

pub fn puppet_warp_pin_angle(session: &PuppetWarpSession, ordinal: u32) -> f64 {
    session
        .mesh
        .as_ref()
        .and_then(|mesh| mesh.pin_angle(ordinal))
        .unwrap_or(0.0)
}

pub fn puppet_warp_bounds(session: &PuppetWarpSession) -> Vec<i32> {
    session.mesh.as_ref().map_or_else(Vec::new, |mesh| {
        let bounds = mesh.bounds();
        vec![bounds.left, bounds.top, bounds.width(), bounds.height()]
    })
}

pub fn puppet_warp_triangles(session: &PuppetWarpSession) -> Vec<f64> {
    session.mesh.as_ref().map_or_else(Vec::new, |mesh| {
        mesh.triangles().flatten().flatten().collect()
    })
}

pub fn puppet_warp_hit(session: &PuppetWarpSession, x: f64, y: f64) -> Vec<f64> {
    let Some(snap) = session.snapshot.as_ref() else {
        return Vec::new();
    };
    let Some([u, v]) = session
        .mesh
        .as_ref()
        .and_then(|mesh| mesh.source_at([x, y]))
    else {
        return Vec::new();
    };
    if (0.0..f64::from(snap.pixels.width)).contains(&u)
        && (0.0..f64::from(snap.pixels.height)).contains(&v)
    {
        vec![u, v]
    } else {
        Vec::new()
    }
}

fn apply_preview_mask(color: &mut PixelBuffer, mask: &PixelBuffer, density: u8) {
    let count = color.pixel_count();
    let alpha = (color.channels as usize - 1) * count;
    for i in 0..count {
        let coverage = 255 - (u32::from(density) * u32::from(255 - mask.data[i]) + 127) / 255;
        color.data[alpha + i] = ((u32::from(color.data[alpha + i]) * coverage + 127) / 255) as u8;
    }
}

pub fn puppet_warp_render(
    session: &PuppetWarpSession,
    x: f64,
    y: f64,
    scale: f64,
    w: i32,
    h: i32,
) -> QImage {
    let render = || {
        let snap = session.snapshot.as_ref()?;
        let mesh = session.mesh.as_ref()?;
        let (w, h) = (u32::try_from(w).ok()?, u32::try_from(h).ok()?);
        let n = u64::from(w) * u64::from(h);
        if n == 0 || n > PREVIEW_PIXELS {
            return None;
        }
        let _held = governor().reserve(n * 16, "Puppet Warp preview").ok()?;
        let mut result =
            mesh.raster(&snap.pixels, snap.binding.mask_default, [x, y], scale, w, h)?;
        if let (Some(mask), Some(density)) = (&snap.linked_mask, snap.linked_mask_density) {
            let warped_mask =
                mesh.raster(mask, snap.binding.linked_mask_default, [x, y], scale, w, h)?;
            apply_preview_mask(&mut result, &warped_mask, density);
        }
        Some(buffer_to_image(&result))
    };
    render().unwrap_or_default()
}

fn offset(local: PsdRect, source: PsdRect) -> Option<PsdRect> {
    Some(PsdRect {
        left: source.left.checked_add(local.left)?,
        top: source.top.checked_add(local.top)?,
        right: source.left.checked_add(local.right)?,
        bottom: source.top.checked_add(local.bottom)?,
    })
}

fn replace(layer: &mut Layer, rect: PsdRect, pixels: PixelBuffer, mask: bool) {
    pictura_codec::edit_layer(layer, |parts| {
        if mask {
            // The mask is present and validated at the transaction boundary.
            if let Some(mask) = &mut parts.mask {
                mask.rect = rect;
                mask.data = Some(pixels.data);
                parts.raw_channels.retain(|channel| channel.id != -2);
                parts.channels.retain(|channel| channel.id != -2);
                if let Some(store) = &mut parts.source_channels {
                    store.planes.retain(|(id, _)| *id != -2);
                }
            }
        } else {
            parts.source_channels = None;
            let colors = i16::from(pixels.channels) - 1;
            let count = pixels.pixel_count();
            parts.rect = rect;
            parts.channels.retain(|channel| channel.id == -2);
            parts.raw_channels.retain(|channel| channel.id == -2);
            for i in 0..pixels.channels as usize {
                let id = if i as i16 == colors { -1 } else { i as i16 };
                let data = pixels.data.as_slice()[i * count..(i + 1) * count].to_vec();
                parts.channels.push(Channel {
                    id,
                    data: data.into(),
                });
            }
        }
    });
}

pub fn puppet_warp_commit(mut view: Pin<&mut PictureView>, session: &PuppetWarpSession) -> bool {
    let (Some(snap), Some(mesh)) = (&session.snapshot, &session.mesh) else {
        return false;
    };
    view.as_mut().settle_pending_composite();
    let valid =
        |rust: &PictureViewRust| target(rust).is_some_and(|(binding, _)| binding == snap.binding);
    if std::ptr::from_ref(&*view).addr() != snap.owner || !valid(view.rust()) {
        return false;
    }
    if mesh.is_identity() {
        return true;
    }
    let bounds = mesh.bounds();
    let Some((w, h)) = dimensions(bounds) else {
        return false;
    };
    let Some(rect) = offset(bounds, snap.binding.rect) else {
        return false;
    };
    let Ok(_held) = governor().reserve(u64::from(w) * u64::from(h) * 16, "Puppet Warp apply")
    else {
        return false;
    };
    let origin = [f64::from(bounds.left), f64::from(bounds.top)];
    let Some(result) = mesh.raster(&snap.pixels, snap.binding.mask_default, origin, 1.0, w, h)
    else {
        return false;
    };
    let linked_result = if let Some(mask) = &snap.linked_mask {
        let Some(result) = mesh.raster(mask, snap.binding.linked_mask_default, origin, 1.0, w, h)
        else {
            return false;
        };
        Some(result)
    } else {
        None
    };
    if rect == snap.binding.rect
        && result.data == snap.pixels.data
        && linked_result
            .as_ref()
            .zip(snap.linked_mask.as_ref())
            .is_none_or(|(result, original)| result.data == original.data)
    {
        return true;
    }
    let outcome = view.edit("Puppet Warp", |rust| {
        if !valid(rust) {
            return Edit::None;
        }
        let Some(layer) = rust
            .session
            .doc
            .as_mut()
            .and_then(|doc| pictura_render::resolve_path_mut(doc, &snap.binding.path))
        else {
            return Edit::None;
        };
        let mask = snap.binding.mask_default.is_some();
        let effects = layer_has_effects(layer)
            || ((mask || snap.linked_mask.is_some())
                && layer.mask.as_ref().is_some_and(|m| m.feather() > 0.0));
        replace(layer, rect, result, mask);
        if let Some(linked_result) = linked_result {
            replace(layer, rect, linked_result, true);
        }
        rust.session.last_transform = None;
        if effects {
            Edit::Full
        } else {
            Edit::regions(vec![snap.binding.rect, rect], true)
        }
    });
    !matches!(outcome, Edit::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::{document_session::DocumentSession, tests::pixel_layer};
    use crate::layer_target::LayerTarget;
    use pictura_core::{Document, LayerMask, LayerStack, Samples, SourceChannels};

    #[test]
    fn linked_mask_union_pads_each_surface() {
        let color_rect = PsdRect {
            left: 0,
            top: 0,
            right: 2,
            bottom: 1,
        };
        let mask_rect = PsdRect {
            left: 1,
            top: 0,
            right: 3,
            bottom: 1,
        };
        let union = PsdRect {
            left: 0,
            top: 0,
            right: 3,
            bottom: 1,
        };
        let mut color = PixelBuffer::new(2, 1, 2);
        color.data.copy_from_slice(&[10, 20, 255, 255]);
        let mut mask = PixelBuffer::new(2, 1, 1);
        mask.data.copy_from_slice(&[30, 40]);
        assert_eq!(
            pad_to_rect(&color, color_rect, union, 0)
                .unwrap()
                .data
                .as_slice(),
            &[10, 20, 0, 255, 255, 0]
        );
        assert_eq!(
            pad_to_rect(&mask, mask_rect, union, 255)
                .unwrap()
                .data
                .as_slice(),
            &[255, 30, 40]
        );
    }

    #[test]
    fn linked_preview_applies_mask_density_to_alpha() {
        let mut color = PixelBuffer::new(2, 1, 4);
        color.data.copy_from_slice(&[1, 2, 3, 4, 5, 6, 200, 200]);
        let mut mask = PixelBuffer::new(2, 1, 1);
        mask.data.copy_from_slice(&[0, 255]);
        apply_preview_mask(&mut color, &mask, 128);
        assert_eq!(&color.data.as_slice()[6..], &[100, 200]);
    }

    #[test]
    fn retained_eight_bit_and_linked_mask_targets() {
        let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
        let mut layer = pixel_layer("px", 16, 16, (40, 120, 200));
        let rect = layer.rect;
        layer.source_channels = Some(SourceChannels::new(
            BitDepth::Eight,
            rect,
            vec![
                (0, Samples::U8(vec![7; 256].into())),
                (-2, Samples::U8(vec![17; 256].into())),
            ],
        ));
        layer.mask = Some(LayerMask {
            rect,
            default_color: 255,
            data: Some(vec![255; 256].into()),
            ..Default::default()
        });
        doc.layers = LayerStack::from_layers(vec![layer]);
        let mut rust = PictureViewRust {
            session: DocumentSession::with_document(doc),
            layer_target: LayerTarget::only("0"),
            ..Default::default()
        };
        let (binding, layer) = target(&rust).unwrap();
        assert_eq!(binding.linked_mask_default, Some(255));
        let (color, mask) = source_pixels(&binding, layer).unwrap();
        assert_eq!(color.data[0], 40);
        assert_eq!(mask.unwrap().data[0], 255);
        rust.mask_edit_target = Some("0".to_string());
        let (binding, layer) = target(&rust).unwrap();
        assert_eq!(binding.mask_default, Some(255));
        assert_eq!(source_pixels(&binding, layer).unwrap().0.data[0], 255);
        let layer = &mut rust.session.doc.as_mut().unwrap().layers[0];
        let mask = PixelBuffer::new(16, 16, 1);
        replace(layer, rect, mask, true);
        let retained = layer.source_channels.as_ref().unwrap();
        assert!(retained.planes.iter().any(|(id, _)| *id == 0));
        assert!(!retained.planes.iter().any(|(id, _)| *id == -2));
        replace(layer, rect, PixelBuffer::new(16, 16, 4), false);
        assert!(layer.source_channels.is_none());
    }
}
