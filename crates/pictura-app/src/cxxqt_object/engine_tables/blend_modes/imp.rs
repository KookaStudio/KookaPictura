use super::super::ffi::BlendModeRow;
use crate::cxxqt_object::qobject::PictureView;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::{BitDepth, BlendMode, ColorMode};

/// The layer modes CS6 offers for `mode` and `depth`, in engine order.
pub(crate) fn allowed_layer_modes(mode: ColorMode, depth: BitDepth) -> Vec<BlendMode> {
    BlendMode::LAYER_MODES
        .iter()
        .copied()
        .filter(|mode_| mode_.allowed(mode, depth))
        .collect()
}

/// The allowed layer-mode keys for the document behind `view`; all 27 without
/// a document.
pub(crate) fn allowed_blend_mode_keys(view: &PictureView) -> QStringList {
    let modes = view.rust().session.doc.as_ref().map_or_else(
        || BlendMode::LAYER_MODES.to_vec(),
        |doc| {
            allowed_layer_modes(
                pictura_render::document_color_mode(doc),
                pictura_render::document_bit_depth(doc),
            )
        },
    );
    let mut keys = QStringList::default();
    for mode in modes {
        keys.append(QString::from(
            String::from_utf8_lossy(&mode.to_psd_key()).as_ref(),
        ));
    }
    keys
}

pub(crate) fn blend_mode_rows() -> Vec<BlendModeRow> {
    BlendMode::LAYER_MODES
        .iter()
        .map(|mode| BlendModeRow {
            key: QString::from(String::from_utf8_lossy(&mode.to_psd_key()).as_ref()),
            name: QString::from(mode.name()),
            has_neutral: pictura_render::neutral_color(*mode).is_some(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_mode_rows_follow_layer_modes() {
        let rows = blend_mode_rows();
        assert_eq!(rows.len(), 27);
        for (row, mode) in rows.iter().zip(BlendMode::LAYER_MODES) {
            let key: [u8; 4] = row.key.to_string().as_bytes().try_into().unwrap();
            assert_eq!(BlendMode::from_psd_key(key), Some(mode));
            assert_eq!(row.name.to_string(), mode.name());
            assert_eq!(
                row.has_neutral,
                pictura_render::neutral_color(mode).is_some()
            );
        }
        assert!(rows.iter().all(|row| row.key.to_string() != "pass"));
        assert_eq!(rows[0].name.to_string(), "Normal");
        assert_eq!(rows[26].name.to_string(), "Luminosity");
    }

    #[test]
    fn allowed_blend_mode_keys_follow_mode_and_depth() {
        let count = |mode, depth| allowed_layer_modes(mode, depth).len();
        assert_eq!(count(ColorMode::Rgb, BitDepth::Eight), 27);
        assert_eq!(count(ColorMode::Rgb, BitDepth::Sixteen), 27);
        assert_eq!(count(ColorMode::Rgb, BitDepth::ThirtyTwo), 15);
        assert_eq!(count(ColorMode::Lab, BitDepth::Eight), 19);
        assert_eq!(count(ColorMode::Bitmap, BitDepth::One), 1);
        assert_eq!(count(ColorMode::Indexed, BitDepth::Eight), 1);

        let hdr = allowed_layer_modes(ColorMode::Rgb, BitDepth::ThirtyTwo);
        assert!(!hdr.contains(&BlendMode::Screen));
        assert!(!hdr.contains(&BlendMode::Overlay));
        assert!(hdr.contains(&BlendMode::Luminosity));

        let lab = allowed_layer_modes(ColorMode::Lab, BitDepth::Eight);
        assert!(!lab.contains(&BlendMode::Subtract));
        assert!(!lab.contains(&BlendMode::Divide));
        assert!(lab.contains(&BlendMode::Screen));
    }
}
