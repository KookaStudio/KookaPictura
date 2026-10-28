//! Smart-object conversion bridge command.
//!
//! `Convert to Smart Object` authors an embedded source from the layer's raster
//! and attaches it while keeping the raster proxy, so the composite is
//! unchanged. Each successful command records exactly one undo state; a refusal
//! records nothing.

use super::helpers::decode_import;
use super::qobject;
use crate::cxxqt_object::edit::{structural, structural_after, Edit};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::SmartObjectKind;

impl qobject::PictureView {
    /// Whether `path` resolves to a convertible raster pixel layer. Read-only.
    pub fn layer_can_convert_to_smart_object(&self, path: &QString) -> bool {
        self.rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::can_convert_to_smart_object(doc, &path.to_string()))
    }

    /// `Convert to Smart Object`: author an embedded source from `path`'s raster
    /// and attach it, keeping the raster proxy. Records one "Convert to Smart
    /// Object" state on success; false (no state) for an ineligible target.
    pub fn convert_to_smart_object(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        let mut changed = false;
        self.as_mut().edit("Convert to Smart Object", |rust| {
            let before = rust
                .session
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::damage::influence(doc, &path));
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            changed = pictura_render::convert_to_smart_object(doc, &path);
            if changed {
                let after = rust
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| pictura_render::damage::influence(doc, &path));
                structural(before, after)
            } else {
                Edit::None
            }
        });
        changed
    }

    /// `File > Place…`: read `file_path`, decode it as a PSD/PSB source, and
    /// append it as a topmost channel-less embedded smart-object layer. Records
    /// one "Place" state on success; empty (no state) when the file is missing,
    /// unreadable, or not a PSD/PSB document.
    pub fn place_smart_object(mut self: Pin<&mut Self>, file_path: &QString) -> QString {
        let path = file_path.to_string();
        let name = std::path::Path::new(&path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = std::fs::read(&path).ok();
        let mut created = String::new();
        self.as_mut().edit("Place", |rust| {
            let (Some(bytes), Some(doc)) = (bytes.as_deref(), rust.session.doc.as_mut()) else {
                return Edit::None;
            };
            let Some(path) = pictura_render::place_smart_object(doc, &name, bytes) else {
                return Edit::None;
            };
            created = path;
            rust.select_only_layer(&created);
            let after = rust
                .session
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::damage::influence(doc, &created));
            structural_after(after.map(|rect| vec![rect]))
        });
        QString::from(created.as_str())
    }

    /// `File > Place…` for a raster image: read `file_path`, probe and decode it
    /// with Qt, append it as a native-size raster layer, convert that layer into
    /// an embedded smart object, recomposite, and record one "Place" state.
    /// Returns the new layer's path, or an empty string recording nothing on any
    /// refusal (no document, missing/unreadable file, unsupported format, over
    /// budget, or failed conversion).
    pub fn place_image(mut self: Pin<&mut Self>, file_path: &QString) -> QString {
        let file = file_path.to_string();
        let name = std::path::Path::new(&file)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(bytes) = super::helpers::safe_io::read_capped(
            &file,
            super::helpers::safe_io::import_file_limit(),
        ) else {
            return QString::default();
        };
        let Some((rgba, width, height)) = decode_import(&bytes) else {
            return QString::default();
        };
        let policy = {
            let mut rust = self.as_mut().rust_mut();
            rust.open_policy_override
                .take()
                .unwrap_or(rust.color_policy)
        };
        let rgba = super::open_jobs::convert_placed_rgba(&bytes, rgba, width, height, policy);
        let mut created = String::new();
        self.as_mut().edit("Place", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let path = pictura_render::add_raster_layer_from_rgba(doc, &name, width, height, &rgba);
            if path.is_empty() {
                return Edit::None;
            }
            if !pictura_render::convert_to_smart_object(doc, &path) {
                // A refusal must leave the document unchanged: undo the
                // append. `add_raster_layer_from_rgba` pushes exactly one
                // top-level layer and a failed conversion adds none.
                let _ = doc.layers.pop();
                return Edit::None;
            }
            created = path;
            rust.select_only_layer(&created);
            let after = rust
                .session
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::damage::influence(doc, &created));
            structural_after(after.map(|rect| vec![rect]))
        });
        QString::from(created.as_str())
    }

    /// Whether `path` resolves to a replaceable smart-object layer. Read-only.
    pub fn layer_can_replace_smart_object_contents(&self, path: &QString) -> bool {
        self.rust().session.doc.as_ref().is_some_and(|doc| {
            pictura_render::can_replace_smart_object_contents(doc, &path.to_string())
        })
    }

    /// `Replace Contents…`: read `file_path`, swap the embedded source of the
    /// smart object at `path`, and keep the layer's transform. Records one
    /// "Replace Contents" state on success; false (no state) when the file is
    /// unreadable, not a PSD/PSB document, or the target is ineligible.
    pub fn replace_smart_object_contents(
        mut self: Pin<&mut Self>,
        path: &QString,
        file_path: &QString,
    ) -> bool {
        self.as_mut()
            .apply_smart_object_source(path, file_path, "Replace Contents")
    }

    /// `Edit Contents`: apply the edited `file_path` as the embedded source of
    /// the smart object at `path`. Shares `replace_smart_object_contents`'s
    /// engine path; records one "Edit Contents" state on success, false (no
    /// state) on the same refusals.
    pub fn commit_smart_object_edit(
        mut self: Pin<&mut Self>,
        path: &QString,
        file_path: &QString,
    ) -> bool {
        self.as_mut()
            .apply_smart_object_source(path, file_path, "Edit Contents")
    }

    /// Whether `path` resolves to a smart-object layer whose embedded payload
    /// parses as a PSD/PSB document, i.e. it can be opened as an in-app editor.
    /// Read-only; mutates nothing.
    pub fn layer_can_edit_smart_object_contents(&self, path: &QString) -> bool {
        self.rust().session.doc.as_ref().is_some_and(|doc| {
            pictura_render::can_edit_smart_object_contents(doc, &path.to_string())
        })
    }

    /// Shared body of Replace Contents and Edit Contents: read `file_path`,
    /// swap the embedded source at `path` with its bytes, and record `label` on
    /// success. A read/parse/eligibility failure returns false and records
    /// nothing.
    fn apply_smart_object_source(
        mut self: Pin<&mut Self>,
        path: &QString,
        file_path: &QString,
        label: &str,
    ) -> bool {
        let file = file_path.to_string();
        let path = path.to_string();
        let name = std::path::Path::new(&file)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = std::fs::read(&file).ok();
        let mut changed = false;
        self.as_mut().edit(label, |rust| {
            let before = rust
                .session
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::damage::influence(doc, &path));
            let (Some(bytes), Some(doc)) = (bytes.as_deref(), rust.session.doc.as_mut()) else {
                return Edit::None;
            };
            changed = pictura_render::replace_smart_object_contents(doc, &path, &name, bytes);
            if changed {
                let after = rust
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| pictura_render::damage::influence(doc, &path));
                structural(before, after)
            } else {
                Edit::None
            }
        });
        changed
    }

    /// Whether `path` resolves to a rasterizable smart-object layer. Read-only.
    pub fn layer_can_rasterize_smart_object(&self, path: &QString) -> bool {
        self.rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::can_rasterize_smart_object(doc, &path.to_string()))
    }

    /// `Rasterize Smart Object`: consume the object at `path`, keeping its raster
    /// proxy (or materializing the embedded source), and drop the preserved
    /// blocks. Records one "Rasterize Smart Object" state on success; false (no
    /// state) for an ineligible target.
    pub fn rasterize_smart_object(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        let mut changed = false;
        self.as_mut().edit("Rasterize Smart Object", |rust| {
            let before = rust
                .session
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::damage::influence(doc, &path));
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            changed = pictura_render::rasterize_smart_object(doc, &path);
            if changed {
                let after = rust
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| pictura_render::damage::influence(doc, &path));
                structural(before, after)
            } else {
                Edit::None
            }
        });
        changed
    }

    /// Self-test probe: `<kind>:<payload-len>` for `path`'s smart object, or
    /// empty when the layer has none. Read-only.
    pub fn layer_smart_object_state(&self, path: &QString) -> QString {
        let state = self
            .rust()
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
            .and_then(|layer| layer.smart_object.as_ref())
            .map(|so| {
                let kind = match so.kind {
                    SmartObjectKind::Embedded => "embedded",
                    SmartObjectKind::External => "external",
                    SmartObjectKind::Alias => "alias",
                    SmartObjectKind::Unresolved => "unresolved",
                };
                format!("{kind}:{}", so.payload().map_or(0, <[u8]>::len))
            })
            .unwrap_or_default();
        QString::from(state.as_str())
    }

    /// Whether `path` resolves to a layer carrying a non-empty embedded payload.
    /// Read-only.
    pub fn layer_can_export_smart_object_contents(&self, path: &QString) -> bool {
        self.rust().session.doc.as_ref().is_some_and(|doc| {
            pictura_render::smart_object_source_bytes(doc, &path.to_string()).is_some()
        })
    }

    /// `Export Contents…`: write the embedded source of the smart object at
    /// `path` to `dest` byte-for-byte. Read-only: records no history state,
    /// clears no link sets, and does not recomposite. Returns true only when the
    /// write succeeds.
    pub fn export_smart_object_contents(&self, path: &QString, dest: &QString) -> bool {
        let Some(bytes) = self
            .rust()
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::smart_object_source_bytes(doc, &path.to_string()))
        else {
            return false;
        };
        write_source_to_path(&dest.to_string(), &bytes)
    }
}

/// Write `bytes` to `dest` byte-for-byte, returning true only when the write
/// succeeds.
pub(crate) fn write_source_to_path(dest: &str, bytes: &[u8]) -> bool {
    std::fs::write(dest, bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::write_source_to_path;

    #[test]
    fn write_source_to_path_reports_success_and_failure() {
        let bytes = b"source-payload-bytes";
        let dest = std::env::temp_dir().join(format!(
            "kooka-pictura-write-source-{}.bin",
            std::process::id()
        ));
        let dest = dest.to_string_lossy().into_owned();
        assert!(write_source_to_path(&dest, bytes));
        assert_eq!(std::fs::read(&dest).expect("read back"), bytes);
        std::fs::remove_file(&dest).expect("cleanup");

        let bad = std::env::temp_dir()
            .join(format!(
                "kooka-pictura-missing-dir-{}/nested/out.bin",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned();
        assert!(!write_source_to_path(&bad, bytes));
        assert!(!std::path::Path::new(&bad).exists());
    }
}
