use super::*;

use super::super::helpers::active_mask_target;
use super::super::impl_filters::{apply_op_active_region, cancel_filter_preview, ActiveOp};
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{BlendMode, PsdRect};
use pictura_render::{
    apply_image_planes, ApplyImageChannel, ApplyImageMode, ApplyImageParams, ApplyImagePlanes,
};
use std::sync::Arc;

pub(crate) struct ApplyImageSnapshot(Option<Arc<ApplyImagePlanes>>);

const SELECTION: i32 = -3;
const ALPHA_BASE: i32 = 100;

fn channel_of(code: i32) -> Option<ApplyImageChannel> {
    match code {
        0 => Some(ApplyImageChannel::Composite),
        1..=3 => Some(ApplyImageChannel::Color(code as u8 - 1)),
        -1 => Some(ApplyImageChannel::Transparency),
        -2 => Some(ApplyImageChannel::LayerMask),
        ALPHA_BASE.. => Some(ApplyImageChannel::Alpha((code - ALPHA_BASE) as usize)),
        _ => None,
    }
}

pub(super) fn apply_image_read(
    source: &PictureView,
    layer: &QString,
    channel: i32,
) -> Box<ApplyImageSnapshot> {
    let rust = source.rust();
    let planes = rust.session.doc.as_ref().and_then(|doc| {
        if channel == SELECTION {
            let selection = rust.session.selection.as_ref()?;
            return ApplyImagePlanes::from_coverage(
                selection.width as usize,
                selection.height as usize,
                selection.data.clone(),
            );
        }
        let path = layer.to_string();
        let path = (!path.is_empty()).then_some(path.as_str());
        apply_image_planes(doc, path, channel_of(channel)?)
    });
    Box::new(ApplyImageSnapshot(planes.map(Arc::new)))
}

pub(super) fn apply_image_same_size(a: &PictureView, b: &PictureView) -> bool {
    let size = |view: &PictureView| {
        view.rust()
            .session
            .doc
            .as_ref()
            .map(|doc| (doc.width, doc.height))
    };
    size(a).is_some() && size(a) == size(b)
}

pub(super) fn apply_image_target_label(view: &PictureView) -> QString {
    let rust = view.rust();
    let name = rust.session.doc.as_ref().and_then(|doc| {
        let path = rust.layer_target.single()?;
        pictura_render::resolve_path(doc, path).map(|layer| layer.name.clone())
    });
    let name = name.unwrap_or_default();
    if active_mask_target(rust).is_some() {
        QString::from(format!("{name} (Layer Mask)").as_str())
    } else {
        QString::from(name.as_str())
    }
}

fn mode_of(mode: &str, scale: f64, offset: i32) -> Option<ApplyImageMode> {
    let scale = scale.clamp(1.0, 2.0) as f32;
    let offset = offset.clamp(-255, 255) as f32;
    match mode {
        "add" => Some(ApplyImageMode::Add { scale, offset }),
        "subtract" => Some(ApplyImageMode::Subtract { scale, offset }),
        key => BlendMode::from_psd_key(key.as_bytes().try_into().ok()?).map(ApplyImageMode::Layer),
    }
}

#[allow(clippy::too_many_arguments)]
fn params(
    source: &ApplyImageSnapshot,
    invert: bool,
    mode: &QString,
    scale: f64,
    offset: i32,
    opacity: i32,
    preserve_transparency: bool,
    mask: &ApplyImageSnapshot,
    mask_invert: bool,
) -> Option<ApplyImageParams> {
    Some(ApplyImageParams {
        source: source.0.clone()?,
        invert,
        mode: mode_of(&mode.to_string(), scale, offset)?,
        opacity: opacity.clamp(0, 100) as f32 / 100.0,
        preserve_transparency,
        mask: mask.0.clone(),
        mask_invert,
    })
}

fn run(
    mut view: Pin<&mut PictureView>,
    params: Option<ApplyImageParams>,
    section: Option<PsdRect>,
    label: Option<&str>,
) -> bool {
    let matches_target = |p: &ApplyImageParams, doc: &pictura_core::Document| {
        let sized = |planes: &ApplyImagePlanes| {
            planes.width == doc.width as usize && planes.height == doc.height as usize
        };
        sized(&p.source) && p.mask.as_deref().is_none_or(sized)
    };
    let region = {
        let mut rust = view.as_mut().rust_mut();
        let usable = params.filter(|p| {
            rust.session
                .doc
                .as_ref()
                .is_some_and(|doc| matches_target(p, doc))
        });
        match usable {
            Some(p) => apply_op_active_region(
                &mut rust,
                &ActiveOp::ApplyImage(p),
                label.is_some(),
                section,
                false,
            ),
            None => {
                rust.filter_error = Some("the Apply Image source is unavailable".to_string());
                None
            }
        }
    };
    let Some(region) = region else {
        {
            let mut rust = view.as_mut().rust_mut();
            let _ = cancel_filter_preview(&mut rust);
        }
        view.as_mut().recomposite();
        return false;
    };
    let outcome = match region {
        Some(rect) => Edit::regions(vec![rect], false),
        None => Edit::Full,
    };
    match label {
        Some(label) => {
            view.as_mut().edit(label, |_| outcome);
        }
        None => {
            view.as_mut().preview(|_| outcome);
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub(super) fn image_apply_image_preview(
    view: Pin<&mut PictureView>,
    source: &ApplyImageSnapshot,
    invert: bool,
    mode: &QString,
    scale: f64,
    offset: i32,
    opacity: i32,
    preserve_transparency: bool,
    mask: &ApplyImageSnapshot,
    mask_invert: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> bool {
    let section = (w > 0 && h > 0).then_some(PsdRect {
        top: y,
        left: x,
        bottom: y + h,
        right: x + w,
    });
    let p = params(
        source,
        invert,
        mode,
        scale,
        offset,
        opacity,
        preserve_transparency,
        mask,
        mask_invert,
    );
    run(view, p, section, None)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn image_apply_image_apply(
    view: Pin<&mut PictureView>,
    source: &ApplyImageSnapshot,
    invert: bool,
    mode: &QString,
    scale: f64,
    offset: i32,
    opacity: i32,
    preserve_transparency: bool,
    mask: &ApplyImageSnapshot,
    mask_invert: bool,
) -> bool {
    let p = params(
        source,
        invert,
        mode,
        scale,
        offset,
        opacity,
        preserve_transparency,
        mask,
        mask_invert,
    );
    run(view, p, None, Some("Apply Image"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_parse_and_clamp() {
        assert_eq!(
            mode_of("add", 9.0, 999),
            Some(ApplyImageMode::Add {
                scale: 2.0,
                offset: 255.0
            })
        );
        assert_eq!(
            mode_of("mul ", 1.0, 0),
            Some(ApplyImageMode::Layer(BlendMode::Multiply))
        );
        assert_eq!(mode_of("nope!", 1.0, 0), None);
    }

    #[test]
    fn channel_codes_map() {
        assert_eq!(channel_of(2), Some(ApplyImageChannel::Color(1)));
        assert_eq!(channel_of(102), Some(ApplyImageChannel::Alpha(2)));
        assert_eq!(channel_of(-100), None);
    }
}
