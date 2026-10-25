//! The blend-mode bridge: the shell builds every layer blend combo from the
//! engine's `BlendMode::LAYER_MODES` instead of a second hand-kept table.

use self::ffi::BlendModeRow;
use cxx_qt_lib::QString;
use pictura_core::BlendMode;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = "pictura"]
    struct BlendModeRow {
        /// The 4-byte PSD key, e.g. `"mul "`.
        key: QString,
        name: QString,
        /// Whether the mode has a neutral colour (New Layer's "Fill with
        /// neutral color" option).
        has_neutral: bool,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The 27 layer modes in engine order; Pass Through is excluded.
        fn blend_mode_rows() -> Vec<BlendModeRow>;
    }
}

fn blend_mode_rows() -> Vec<BlendModeRow> {
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
}
