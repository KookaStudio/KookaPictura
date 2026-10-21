//! Host fonts for the Type tools: faces the shell registers by family (Qt
//! resolves the family and hands over its tables), so type renders, lays out,
//! and places its caret in the font the options bar names. Anything not
//! registered falls back to the bundled face.
//!
//! ponytail: a registered font's bytes and parsed face live for the process —
//! bounded by the families a user actually picks.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::text_render::{bundled, BundledText};

struct Registered {
    family: String,
    postscript: String,
    face: BundledText,
}

fn registry() -> &'static RwLock<HashMap<String, &'static Registered>> {
    static REGISTRY: OnceLock<RwLock<HashMap<String, &'static Registered>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Family and PostScript names meet on one key: case and spacing ignored.
fn key(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn lookup(name: &str) -> Option<&'static Registered> {
    registry().read().ok()?.get(&key(name)).copied()
}

/// Register `bytes` (an sfnt) as `family`, also reachable by its PostScript
/// name. True when the family is (already) registered; false for bytes that do
/// not parse.
pub fn register_font(family: &str, bytes: Vec<u8>) -> bool {
    if family.trim().is_empty() {
        return false;
    }
    if lookup(family).is_some_and(|r| key(&r.family) == key(family)) {
        return true;
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
    let Ok(mut map) = registry().write() else {
        return false;
    };
    map.insert(key(&entry.postscript), entry);
    map.insert(key(family), entry);
    true
}

/// The face for a family or PostScript name, else the bundled face.
pub(crate) fn face_for(name: &str) -> Option<&'static BundledText> {
    lookup(name).map(|r| &r.face).or_else(bundled)
}

/// Which of the Character panel's eight OpenType features the face for
/// `name` carries, one bit each in panel order (bit 0 Standard Ligatures …
/// bit 7 Fractions); 0 when no face resolves.
pub fn opentype_features(name: &str) -> u32 {
    face_for(name).map_or(0, crate::type_style::opentype_support)
}

/// A registered family's PostScript name, which `TySh` should carry.
pub fn font_postscript_name(name: &str) -> Option<String> {
    lookup(name).map(|r| r.postscript.clone())
}

/// The registered family a family or PostScript name belongs to.
pub fn font_family(name: &str) -> Option<String> {
    lookup(name).map(|r| r.family.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIBERATION: &[u8] = include_bytes!("../assets/LiberationSans-Regular.ttf");

    #[test]
    fn registered_fonts_answer_to_family_and_postscript_names() {
        assert!(!register_font("Bogus", vec![1, 2, 3]));
        assert!(font_family("Bogus").is_none());
        assert!(register_font("My Test Sans", LIBERATION.to_vec()));
        assert!(register_font("my test sans", LIBERATION.to_vec()));
        assert_eq!(
            font_postscript_name("My Test Sans").as_deref(),
            Some("LiberationSans")
        );
        assert_eq!(
            font_family("LiberationSans").as_deref(),
            Some("My Test Sans")
        );
        assert_eq!(font_family("MyTestSans").as_deref(), Some("My Test Sans"));
        assert!(
            face_for("Unregistered Family").is_some(),
            "falls back to the bundled face"
        );
    }
}
