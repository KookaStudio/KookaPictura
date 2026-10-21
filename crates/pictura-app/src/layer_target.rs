//! The per-document layer target: the selected layer paths and the active path
//! (the anchor). One value owns what tools, commands, the panel, and history
//! read; paths stay the document's addressing scheme.

use pictura_core::Document;

/// Selected layer paths plus the active path. `active` is always one of
/// `selected`, or `None` when nothing is selected.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LayerTarget {
    selected: Vec<String>,
    active: Option<String>,
}

impl LayerTarget {
    /// The single tool-edit target, present only when exactly one path is
    /// selected.
    pub(crate) fn single(&self) -> Option<&str> {
        (self.selected.len() == 1).then(|| self.selected[0].as_str())
    }

    /// The panel's current row, always a selected path (or empty).
    pub(crate) fn anchor(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub(crate) fn selected(&self) -> &[String] {
        &self.selected
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    /// Replace the target with one layer; an empty path clears it.
    #[cfg(test)]
    pub(crate) fn only(path: &str) -> Self {
        let mut target = Self::default();
        target.select_only(path);
        target
    }

    pub(crate) fn select_only(&mut self, path: &str) {
        if path.is_empty() {
            self.clear();
        } else {
            self.selected.clear();
            self.selected.push(path.to_string());
            self.active = Some(path.to_string());
        }
    }

    /// Replace the target with `paths`, resolving them against `doc` when one
    /// is available. Duplicate, malformed, and missing paths are dropped; the
    /// anchor falls back to the first survivor.
    pub(crate) fn select(
        &mut self,
        paths: impl IntoIterator<Item = String>,
        anchor: Option<String>,
        doc: Option<&Document>,
    ) {
        self.selected.clear();
        for path in paths {
            if path.is_empty() || self.selected.iter().any(|kept| kept == &path) {
                continue;
            }
            if doc.is_some_and(|doc| pictura_render::resolve_path(doc, &path).is_none()) {
                continue;
            }
            self.selected.push(path);
        }
        self.active = anchor
            .filter(|anchor| self.selected.iter().any(|path| path == anchor))
            .or_else(|| self.selected.first().cloned());
    }

    pub(crate) fn clear(&mut self) {
        self.selected.clear();
        self.active = None;
    }

    /// Drop paths that no longer resolve, then repair the anchor.
    pub(crate) fn retain_resolving(&mut self, doc: &Document) {
        self.selected
            .retain(|path| pictura_render::resolve_path(doc, path).is_some());
        if self
            .active
            .as_ref()
            .is_some_and(|active| !self.selected.iter().any(|path| path == active))
        {
            self.active = self.selected.first().cloned();
        }
    }

    /// The target a freshly opened document starts with: the decoded active
    /// layer when it still resolves, else the topmost pixel layer, else the
    /// topmost top-level layer.
    pub(crate) fn for_opened(doc: &Document, active: Option<String>) -> Self {
        let mut target = Self::default();
        if let Some(active) =
            active.filter(|path| pictura_render::resolve_path(doc, path).is_some())
        {
            target.select_only(&active);
            return target;
        }
        let topmost_pixel = pictura_render::flatten_rows(doc)
            .into_iter()
            .map(|(path, _)| path)
            .find(|path| {
                pictura_render::resolve_path(doc, path)
                    .is_some_and(|layer| layer.adjustment.is_none() && !layer.is_group)
            });
        if let Some(path) = topmost_pixel {
            target.select_only(&path);
        } else if let Some(index) = doc.layers.len().checked_sub(1) {
            target.select_only(&index.to_string());
        }
        target
    }
}

#[cfg(test)]
mod tests {
    use super::LayerTarget;
    use pictura_core::{BitDepth, ColorMode, Document, Layer, LayerStack};

    fn pixel(name: &str) -> Layer {
        Layer {
            name: name.to_string(),
            rect: pictura_core::PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            },
            ..Layer::default()
        }
    }

    fn doc(names: &[&str]) -> Document {
        let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(names.iter().map(|name| pixel(name)).collect());
        doc
    }

    #[test]
    fn single_only_when_one() {
        let doc = doc(&["base", "top"]);
        let mut target = LayerTarget::default();
        target.select(["0".to_string()], None, Some(&doc));
        assert_eq!(target.single(), Some("0"));
        assert_eq!(target.anchor(), Some("0"));
        target.select(
            ["0".to_string(), "1".to_string()],
            Some("1".to_string()),
            Some(&doc),
        );
        assert_eq!(target.single(), None);
        assert_eq!(target.anchor(), Some("1"));
        assert_eq!(target.selected(), ["0".to_string(), "1".to_string()]);
    }

    #[test]
    fn stale_paths_are_dropped() {
        let doc = doc(&["base", "top"]);
        let mut target = LayerTarget::default();
        target.select(
            [
                "0".to_string(),
                "9".to_string(),
                "0".to_string(),
                "x".to_string(),
            ],
            Some("9".to_string()),
            Some(&doc),
        );
        assert_eq!(target.selected(), ["0".to_string()]);
        assert_eq!(target.anchor(), Some("0"));
        assert_eq!(target.single(), Some("0"));
    }

    #[test]
    fn stale_anchor_coerces_to_a_survivor() {
        let doc = doc(&["base", "middle", "top"]);
        let mut target = LayerTarget::default();
        target.select(
            ["0".to_string(), "2".to_string()],
            Some("2".to_string()),
            Some(&doc),
        );
        target.retain_resolving(&doc_without(&doc, "2"));
        assert_eq!(target.selected(), ["0".to_string()]);
        assert_eq!(target.anchor(), Some("0"));
    }

    fn doc_without(doc: &Document, _removed: &str) -> Document {
        let mut next = doc.clone();
        next.layers = LayerStack::from_layers(next.layers.iter().take(1).cloned().collect());
        next
    }
}

#[cfg(test)]
mod opened_tests {
    use super::LayerTarget;
    use pictura_core::{BitDepth, ColorMode, Document, Layer, LayerStack};

    fn layer(name: &str, group: bool) -> Layer {
        Layer {
            name: name.to_string(),
            is_group: group,
            rect: pictura_core::PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            },
            ..Layer::default()
        }
    }

    fn doc(layers: Vec<Layer>) -> Document {
        let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(layers);
        doc
    }

    #[test]
    fn open_target_prefers_the_topmost_pixel_layer() {
        let doc = doc(vec![layer("pixel", false), layer("group", true)]);
        let target = LayerTarget::for_opened(&doc, None);
        assert_eq!(target.single(), Some("0"));
    }

    #[test]
    fn open_target_falls_back_to_the_top_layer() {
        let doc = doc(vec![layer("group", true)]);
        let target = LayerTarget::for_opened(&doc, None);
        assert_eq!(target.single(), Some("0"));
    }

    #[test]
    fn open_target_finds_a_pixel_inside_the_top_group() {
        let mut group = layer("group", true);
        group.children = LayerStack::from_layers(vec![layer("child", false)]);
        let doc = doc(vec![layer("bottom", false), group]);
        let target = LayerTarget::for_opened(&doc, None);
        assert_eq!(target.single(), Some("1/0"));
    }
}
