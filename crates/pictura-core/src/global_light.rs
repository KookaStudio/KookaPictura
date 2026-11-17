//! The document's global light: the angle and altitude that layer effects with
//! `Use Global Light` read (PSD image resources 1037 and 1049).

/// The document-wide light angle and altitude, in degrees.
///
/// Photoshop's default is 30/30. The values ride the history snapshot and are
/// emitted to the PSD image-resource section on save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalLight {
    pub angle: i32,
    pub altitude: i32,
}

impl Default for GlobalLight {
    fn default() -> Self {
        Self {
            angle: 30,
            altitude: 30,
        }
    }
}
