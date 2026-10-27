//! Host fonts for the Type tools: faces the shell registers by family and
//! style (Qt resolves the face and hands over its tables), so type renders,
//! lays out, and places its caret in the font the Character panel names.
//! Anything not registered falls back to the bundled face.
//!
//! ponytail: a registered font's bytes and parsed face live for the process —
//! bounded by the faces a user actually picks.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};

use crate::text_render::{bundled, BundledText};

struct Registered {
    family: String,
    postscript: String,
    face: BundledText,
}

#[derive(Default)]
struct Registry {
    /// `family|style` keys, both normalized.
    faces: HashMap<String, &'static Registered>,
    /// The family's regular (empty-style) face.
    regular: HashMap<String, &'static Registered>,
    /// A family's first registered face, whatever its style.
    families: HashMap<String, &'static Registered>,
    /// PostScript names are unique per face, so they need no style key.
    postscript: HashMap<String, &'static Registered>,
    /// Registration order, for deterministic fallback selection.
    order: Vec<&'static Registered>,
}

/// Missing Glyph Protection is application-wide, not document or history state.
static MISSING_GLYPH_PROTECTION: AtomicBool = AtomicBool::new(true);

/// Bumped whenever a face is registered, so caches keyed on text invalidate.
static REGISTRY_GENERATION: AtomicU64 = AtomicU64::new(0);

/// The cache identity of the current font registry.
pub(crate) fn generation() -> u64 {
    REGISTRY_GENERATION.load(Ordering::Relaxed)
}

/// Turn per-character fallback on or off for every document.
pub fn set_missing_glyph_protection(on: bool) {
    MISSING_GLYPH_PROTECTION.store(on, Ordering::Relaxed);
}

/// Whether per-character fallback is on.
pub fn missing_glyph_protection() -> bool {
    MISSING_GLYPH_PROTECTION.load(Ordering::Relaxed)
}

fn registry() -> &'static RwLock<Registry> {
    static REGISTRY: OnceLock<RwLock<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Registry::default()))
}

/// Family, style, and PostScript names meet on one normalized key: case and
/// spacing ignored.
fn key(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn face_key(family: &str, style: &str) -> String {
    format!("{}|{}", key(family), key(style))
}

fn lookup(family: &str, style: &str) -> Option<&'static Registered> {
    let registry = registry().read().ok()?;
    if let Some(entry) = registry.faces.get(&face_key(family, style)).copied() {
        return Some(entry);
    }
    if let Some(entry) = registry.regular.get(&key(family)).copied() {
        return Some(entry);
    }
    if let Some(entry) = registry.postscript.get(&key(family)).copied() {
        return Some(entry);
    }
    registry.families.get(&key(family)).copied()
}

/// Register `bytes` (an sfnt) as `family` in `style`, also reachable by its
/// PostScript name. True when the face is (already) registered; false for
/// bytes that do not parse.
pub fn register_font(family: &str, style: &str, bytes: Vec<u8>) -> bool {
    if family.trim().is_empty() {
        return false;
    }
    {
        let Ok(registry) = registry().read() else {
            return false;
        };
        if registry.faces.contains_key(&face_key(family, style)) {
            return true;
        }
    }
    let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let Some(face) = BundledText::from_bytes(bytes) else {
        return false;
    };
    let postscript = face
        .postscript_name()
        .unwrap_or_else(|| family.split_whitespace().collect());
    let entry: &'static Registered = Box::leak(Box::new(Registered {
        family: family.to_string(),
        postscript,
        face,
    }));
    let Ok(mut registry) = registry().write() else {
        return false;
    };
    registry.faces.insert(face_key(family, style), entry);
    let normalized_style = key(style);
    if normalized_style.is_empty() || normalized_style == "regular" {
        registry.regular.insert(key(family), entry);
    }
    registry.families.entry(key(family)).or_insert(entry);
    registry.postscript.insert(key(&entry.postscript), entry);
    registry.order.push(entry);
    REGISTRY_GENERATION.fetch_add(1, Ordering::Relaxed);
    true
}

/// The face for a family (or PostScript name) and style, else the family's
/// regular face, else the bundled face.
pub(crate) fn face_for(family: &str, style: &str) -> Option<&'static BundledText> {
    lookup(family, style).map(|r| &r.face).or_else(bundled)
}

/// Every registered face in registration order, the active face first. The
/// caller filters by glyph coverage.
///
/// ponytail: one process-wide order, not a per-family fallback list; the shell
/// registers one fallback family per common script once.
pub(crate) fn faces_in_order(active: &BundledText) -> Vec<&BundledText> {
    let mut out = vec![active];
    let Ok(registry) = registry().read() else {
        return out;
    };
    for entry in &registry.order {
        if std::ptr::eq(&entry.face, active) {
            continue;
        }
        out.push(&entry.face);
    }
    out
}

/// Which of the Character panel's eight OpenType features the face for
/// `family`/`style` carries, one bit each in panel order (bit 0 Standard
/// Ligatures … bit 7 Fractions); 0 when no face resolves.
pub fn opentype_features(family: &str, style: &str) -> u32 {
    face_for(family, style).map_or(0, crate::type_style::opentype_support)
}

/// Whether the face for `family`/`style` has outline glyphs (`glyf`, `CFF `
/// or `CFF2`), which Warp Text, Create Work Path, and Convert to Shape need; a
/// bitmap-only face has none. False when no face resolves.
pub fn font_has_outlines(family: &str, style: &str) -> bool {
    face_for(family, style).is_some_and(face_has_outlines)
}

pub(crate) fn face_has_outlines(face: &BundledText) -> bool {
    let raw = face.ttf().raw_face();
    [b"glyf", b"CFF ", b"CFF2"].iter().any(|tag| {
        raw.table(rustybuzz::ttf_parser::Tag::from_bytes(tag))
            .is_some()
    })
}

/// A registered face's PostScript name, which `TySh` should carry.
pub fn font_postscript_name(family: &str, style: &str) -> Option<String> {
    lookup(family, style).map(|r| r.postscript.clone())
}

/// The registered family a family or PostScript name belongs to.
pub fn font_family(name: &str) -> Option<String> {
    let registry = registry().read().ok()?;
    registry
        .families
        .get(&key(name))
        .or_else(|| registry.postscript.get(&key(name)))
        .map(|r| r.family.clone())
}

#[cfg(test)]
pub(crate) static FALLBACK_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    const LIBERATION: &[u8] = include_bytes!("../assets/LiberationSans-Regular.ttf");

    #[test]
    fn registered_fonts_answer_to_family_and_postscript_names() {
        assert!(!register_font("Bogus", "", vec![1, 2, 3]));
        assert!(font_family("Bogus").is_none());
        assert!(register_font("My Test Sans", "", LIBERATION.to_vec()));
        assert!(register_font("my test sans", "", LIBERATION.to_vec()));
        assert_eq!(
            font_postscript_name("My Test Sans", "").as_deref(),
            Some("LiberationSans")
        );
        assert_eq!(
            font_family("LiberationSans").as_deref(),
            Some("My Test Sans")
        );
        assert_eq!(font_family("MyTestSans").as_deref(), Some("My Test Sans"));
        assert!(
            face_for("Unregistered Family", "").is_some(),
            "falls back to the bundled face"
        );
    }

    fn register_style_faces() {
        let bold = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/NotoSans-Bold.ttf"
        ))
        .to_vec();
        let regular = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        assert!(register_font("Style Support", "Bold", bold.clone()));
        assert!(register_font("Style Support", "Regular", regular.clone()));
        assert!(register_font("Style Render", "Bold", bold));
        assert!(register_font("Style Render", "Regular", regular));
    }

    #[test]
    fn style_support_follows_selected_face() {
        register_style_faces();
        let bold = face_for("Style Support", "Bold").expect("bold face");
        assert_eq!(
            opentype_features("Style Support", "Bold"),
            crate::type_style::opentype_support(bold)
        );
    }

    #[test]
    fn bold_style_renders_different_face() {
        register_style_faces();
        assert_ne!(
            font_postscript_name("Style Render", "Bold"),
            font_postscript_name("Style Render", "Regular")
        );
    }

    #[test]
    fn unregistered_style_falls_back_to_regular() {
        register_style_faces();
        let regular = face_for("Style Render", "Regular").expect("regular face");
        let black = face_for("Style Render", "Black").expect("fallback face");
        assert!(std::ptr::eq(regular, black));
    }

    #[test]
    fn style_name_matching_ignores_case_and_space() {
        register_style_faces();
        assert_eq!(
            font_postscript_name("style render", "bold"),
            font_postscript_name("Style Render", "Bold")
        );
    }

    #[test]
    fn fallback_prefers_active_then_registration_order() {
        let _guard = FALLBACK_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let noto = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        let bold = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/NotoSans-Bold.ttf"
        ))
        .to_vec();
        assert!(register_font("Fallback One", "", noto));
        assert!(register_font("Fallback Two", "", bold));
        let active = face_for("Liberation Sans", "").expect("active");
        let faces = faces_in_order(active);
        let a = faces.iter().find(|face| face.has_glyph('A')).expect("A");
        assert!(
            std::ptr::eq(*a, active),
            "the active face wins when it covers"
        );
        let het = '\u{370}';
        let first = faces
            .iter()
            .find(|face| face.has_glyph(het))
            .expect("fallback");
        assert!(
            !std::ptr::eq(*first, active),
            "an uncovered glyph uses a fallback"
        );
    }

    #[test]
    fn style_faces_resolve_and_fall_back() {
        let bold = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/NotoSans-Bold.ttf"
        ))
        .to_vec();
        let regular = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        assert!(register_font("Style Test", "Bold", bold));
        assert!(register_font("Style Test", "Regular", regular));
        assert!(font_postscript_name("Style Test", "Bold").is_some());
        assert!(face_for("Style Test", "Bold").is_some());
        assert!(
            face_for("Style Test", "Black").is_some(),
            "falls back to Regular"
        );
        assert_eq!(
            font_postscript_name("Style Test", "Bold"),
            font_postscript_name("Style Test", "Bold")
        );
    }
}
