use super::*;
use pictura_core::{DocumentMeasurementScale, Edition, MeasurementScale};

fn custom_scale(view: &PictureView) -> Option<&MeasurementScale> {
    view.rust().session.doc.as_ref()?.measurement_scale.custom()
}

pub(in crate::cxxqt_object::annotations) fn measurement_scale_values(
    view: &PictureView,
) -> Vec<f64> {
    custom_scale(view).map_or_else(Vec::new, |scale| {
        vec![f64::from(scale.pixel_length()), scale.logical_length()]
    })
}

pub(in crate::cxxqt_object::annotations) fn measurement_scale_units(view: &PictureView) -> QString {
    QString::from(custom_scale(view).map_or("px", MeasurementScale::units))
}

pub(in crate::cxxqt_object::annotations) fn measurement_scale_text(view: &PictureView) -> QString {
    if !view.has_document() {
        return QString::default();
    }
    custom_scale(view).map_or_else(
        || QString::from("1 px = 1 px"),
        |scale| {
            QString::from(
                format!(
                    "{} px = {} {}",
                    scale.pixel_length(),
                    scale.logical_length(),
                    scale.units()
                )
                .as_str(),
            )
        },
    )
}

pub(in crate::cxxqt_object::annotations) fn calibrated_ruler_length(view: &PictureView) -> f64 {
    custom_scale(view)
        .zip(view.rust().ruler)
        .map_or(f64::NAN, |(scale, ruler)| {
            scale.logical_distance(ruler.measure().distance)
        })
}

pub(in crate::cxxqt_object::annotations) fn measurement_scale_needs_recalibration(
    view: &PictureView,
) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| doc.measurement_scale.needs_recalibration())
}

pub(in crate::cxxqt_object::annotations) fn measurement_scale_valid(
    pixels: i32,
    logical: f64,
    units: &QString,
) -> bool {
    MeasurementScale::new(pixels, logical, &units.to_string()).is_some()
}

fn change_scale(
    view: Pin<&mut PictureView>,
    change: impl FnOnce(&mut DocumentMeasurementScale) -> bool,
) -> bool {
    if !Edition::from_env().is_extended()
        || view.rust().modal_transform_blocks_edit()
        || view.rust().document_job_pending()
    {
        return false;
    }
    matches!(
        view.edit("Set Measurement Scale", |rust| {
            if change(
                &mut rust
                    .session
                    .doc
                    .as_mut()
                    .expect("open document")
                    .measurement_scale,
            ) {
                Edit::Meta
            } else {
                Edit::None
            }
        }),
        Edit::Meta
    )
}

pub(in crate::cxxqt_object::annotations) fn apply_measurement_scale(
    view: Pin<&mut PictureView>,
    pixels: i32,
    logical: f64,
    units: &QString,
) -> bool {
    let Some(scale) = MeasurementScale::new(pixels, logical, &units.to_string()) else {
        return false;
    };
    change_scale(view, |state| state.apply(scale))
}

pub(in crate::cxxqt_object::annotations) fn reset_measurement_scale(
    view: Pin<&mut PictureView>,
) -> bool {
    change_scale(view, DocumentMeasurementScale::reset)
}
