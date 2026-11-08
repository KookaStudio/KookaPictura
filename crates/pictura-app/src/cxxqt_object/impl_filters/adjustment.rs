use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers::*;
use crate::cxxqt_object::helpers_composite::*;
use crate::cxxqt_object::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

impl qobject::PictureView {
    pub fn add_adjustment(mut self: Pin<&mut Self>, kind: &QString) -> bool {
        if self.rust().modal_transform_blocks_edit() {
            return false;
        }
        let (anchor, use_mask, mask, cmyk) = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            let mask = if rust.add_mask_by_default {
                Some(default_layer_mask(
                    doc,
                    rust.session
                        .selection
                        .as_ref()
                        .map(|selection| selection_to_mask(selection, doc)),
                ))
            } else {
                None
            };
            (
                rust.layer_target.anchor().unwrap_or("").to_string(),
                rust.add_mask_by_default,
                mask,
                pictura_render::document_color_mode(doc) == pictura_core::ColorMode::Cmyk,
            )
        };
        let Some(layer) = adjustment_layer_in_mode(&kind.to_string(), mask, cmyk) else {
            return false;
        };
        let mut created = String::new();
        let pushed = match self.as_mut().rust_mut().session.doc.as_mut() {
            Some(doc) => {
                created = pictura_render::insert_layer(doc, &anchor, layer);
                !created.is_empty()
            }
            None => false,
        };
        if pushed {
            let region = self
                .rust()
                .session
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::damage::influence(doc, &created));
            // A masked adjustment is confined to its mask; an unmasked one spans
            // the canvas and falls back to a full recomposite.
            let outcome = match region {
                // Creation adds a row, so the panel must refresh even when
                // the recomposite is limited to the new mask's region.
                Some(rect) => Edit::regions(vec![rect], true),
                None => Edit::Full,
            };
            self.as_mut().edit("Adjustment", |rust| {
                // Inserting below the top shifts every later path, so the
                // path-keyed link sets no longer name the same layers.
                rust.link_sets.clear();
                rust.link_remap = None;
                rust.select_only_layer(&created);
                if use_mask {
                    // The created mask becomes the edit target, as CS6 selects
                    // its thumbnail; mask solo stays off.
                    let has_mask = rust
                        .session
                        .doc
                        .as_ref()
                        .and_then(|doc| pictura_render::resolve_path(doc, &created))
                        .is_some_and(|layer| layer.mask.is_some());
                    if has_mask {
                        rust.mask_edit_target = Some(created.clone());
                    }
                }
                // A recomposite is not a structural edit, so clear the solo
                // snapshot explicitly.
                rust.solo_snapshot = None;
                outcome
            });
        }
        pushed
    }
}
