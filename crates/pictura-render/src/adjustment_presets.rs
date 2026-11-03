//! Adjustment presets: the kind facts the shell keys a preset list by, and the
//! per-user store that survives a restart.
//!
//! Built-in looks live with the shell's dialog tables; this module owns the
//! user-saved blocks. The store is one text file per adjustment kind under the
//! XDG user data directory, so a preset is never part of the PSD.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use pictura_adjust::Adjustment;
use pictura_core::AdjustmentData;

use crate::decode_adjustment;

/// The adjustment kinds CS6 lets the user save a preset for.
pub const SAVEABLE_KINDS: [&str; 7] = [
    "levels",
    "curves",
    "exposure",
    "hue-saturation",
    "black-white",
    "channel-mixer",
    "selective-color",
];

/// The adjustment kinds whose Properties page carries the Preset menu.
pub const MENU_KINDS: [&str; 5] = [
    "levels",
    "curves",
    "photo-filter",
    "selective-color",
    "vibrance",
];

/// The reserved names a user preset may not take.
const RESERVED_NAMES: [&str; 2] = ["Default", "Custom"];

/// A preset name longer than this is truncated.
const MAX_NAME_CHARS: usize = 80;

/// The catalogue kind of `data`'s adjustment, or `None` when it does not
/// decode.
pub fn kind_of(data: &AdjustmentData) -> Option<&'static str> {
    Some(match decode_adjustment(data)? {
        Adjustment::Levels(_) => "levels",
        Adjustment::Curves(_) => "curves",
        Adjustment::BrightnessContrast(_) => "brightness-contrast",
        Adjustment::Exposure(_) => "exposure",
        Adjustment::Vibrance(_) => "vibrance",
        Adjustment::HueSaturation(_) => "hue-saturation",
        Adjustment::ColorBalance(_) => "color-balance",
        Adjustment::BlackWhite(_) => "black-white",
        Adjustment::PhotoFilter(_) => "photo-filter",
        Adjustment::ChannelMixer(_) => "channel-mixer",
        Adjustment::ColorLookup(_) => "color-lookup",
        Adjustment::Invert => "invert",
        Adjustment::Posterize(_) => "posterize",
        Adjustment::Threshold(_) => "threshold",
        Adjustment::GradientMap(_) => "gradient-map",
        Adjustment::SelectiveColor(_) => "selective-color",
        Adjustment::ShadowsHighlights(_) => "shadows-highlights",
        Adjustment::Desaturate => "desaturate",
        Adjustment::Equalize => "equalize",
        Adjustment::Auto(_) => "auto",
        Adjustment::GradientFill(_) => "gradient",
        Adjustment::SolidFill(_) => "solid-color",
        Adjustment::PatternFill(_) => "pattern",
        Adjustment::ReplaceColor(_) => "replace-color",
        Adjustment::MatchColor(_) => "match-color",
    })
}

/// Whether `kind` is in the CS6 save whitelist.
pub fn is_saveable(kind: &str) -> bool {
    SAVEABLE_KINDS.contains(&kind)
}

/// Whether `kind`'s Properties page carries the preset menu.
pub fn supports_menu(kind: &str) -> bool {
    MENU_KINDS.contains(&kind)
}

/// Whether two blocks decode to the same adjustment. Two encodings of the same
/// settings therefore match, which is what the Preset menu needs to name the
/// current preset.
pub fn blocks_match(a: &AdjustmentData, b: &AdjustmentData) -> bool {
    match (decode_adjustment(a), decode_adjustment(b)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// A store name: trimmed, control characters replaced, and the reserved names
/// refused.
pub fn sanitize_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() || RESERVED_NAMES.contains(&cleaned) {
        return None;
    }
    Some(cleaned.chars().take(MAX_NAME_CHARS).collect())
}

/// The user store root: `$XDG_DATA_HOME/kooka-pictura/Presets/Adjustments`,
/// or `$HOME/.local/share/...` when `XDG_DATA_HOME` is unset.
pub fn presets_root() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(
        base.join("kooka-pictura")
            .join("Presets")
            .join("Adjustments"),
    )
}

/// Saved names for `kind`, in store order.
pub fn saved_names(kind: &str) -> Vec<String> {
    presets_root()
        .map(|root| saved_names_at(&root, kind))
        .unwrap_or_default()
}

/// The block saved as `name`, or `None`.
pub fn saved_block(kind: &str, name: &str) -> Option<AdjustmentData> {
    saved_block_at(&presets_root()?, kind, name)
}

/// Save `block` as `name` for `kind`; `false` when the name, kind, or block is
/// refused or the write fails.
pub fn save(kind: &str, name: &str, block: &AdjustmentData) -> bool {
    presets_root().is_some_and(|root| save_at(&root, kind, name, block))
}

/// Delete `name` from `kind`; `false` when it was absent or the write fails.
pub fn delete(kind: &str, name: &str) -> bool {
    presets_root().is_some_and(|root| delete_at(&root, kind, name))
}

/// Saved names under `root`, in store order.
pub fn saved_names_at(root: &Path, kind: &str) -> Vec<String> {
    read_store(root, kind)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// The block saved as `name` under `root`.
pub fn saved_block_at(root: &Path, kind: &str, name: &str) -> Option<AdjustmentData> {
    let name = sanitize_name(name)?;
    let (_, bytes) = read_store(root, kind)
        .into_iter()
        .find(|(saved, _)| saved == &name)?;
    block_from_bytes(&bytes)
}

/// Save `block` as `name` under `root`.
pub fn save_at(root: &Path, kind: &str, name: &str, block: &AdjustmentData) -> bool {
    let Some(name) = sanitize_name(name) else {
        return false;
    };
    if kind_of(block) != Some(kind) {
        return false;
    }
    let mut records = read_store(root, kind);
    let bytes = block_bytes(block);
    match records.iter_mut().find(|(saved, _)| saved == &name) {
        Some((_, slot)) => *slot = bytes,
        None => records.push((name, bytes)),
    }
    write_store(root, kind, &records)
}

/// Delete `name` under `root`.
pub fn delete_at(root: &Path, kind: &str, name: &str) -> bool {
    let Some(name) = sanitize_name(name) else {
        return false;
    };
    let mut records = read_store(root, kind);
    let before = records.len();
    records.retain(|(saved, _)| saved != &name);
    before != records.len() && write_store(root, kind, &records)
}

/// `kind`'s store file, or `None` for a kind that is not a preset kind.
fn store_path(root: &Path, kind: &str) -> Option<PathBuf> {
    (is_known_kind(kind)).then(|| root.join(format!("{kind}.presets")))
}

fn is_known_kind(kind: &str) -> bool {
    MENU_KINDS.contains(&kind)
        || SAVEABLE_KINDS.contains(&kind)
        || matches!(
            kind,
            "exposure" | "hue-saturation" | "black-white" | "channel-mixer"
        )
}

fn block_bytes(data: &AdjustmentData) -> Vec<u8> {
    data.key
        .iter()
        .copied()
        .chain(data.data.iter().copied())
        .collect()
}

fn block_from_bytes(bytes: &[u8]) -> Option<AdjustmentData> {
    Some(AdjustmentData {
        key: bytes.get(..4)?.try_into().ok()?,
        data: bytes.get(4..)?.to_vec().into(),
    })
}

/// Read a store; a missing, unreadable, or malformed file reads as empty.
fn read_store(root: &Path, kind: &str) -> Vec<(String, Vec<u8>)> {
    let Some(path) = store_path(root, kind) else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut records: Vec<(String, Vec<u8>)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, hex)) = line.split_once('\t') else {
            continue;
        };
        let Some(name) = sanitize_name(name) else {
            continue;
        };
        let Some(bytes) = hex_decode(hex) else {
            continue;
        };
        let Some(data) = block_from_bytes(&bytes) else {
            continue;
        };
        if kind_of(&data) != Some(kind) {
            continue;
        }
        match records.iter_mut().find(|(saved, _)| saved == &name) {
            Some((_, slot)) => *slot = bytes,
            None => records.push((name, bytes)),
        }
    }
    records
}

/// Write a store atomically: a temporary file in the same directory is renamed
/// over the target.
fn write_store(root: &Path, kind: &str, records: &[(String, Vec<u8>)]) -> bool {
    let Some(path) = store_path(root, kind) else {
        return false;
    };
    if fs::create_dir_all(root).is_err() {
        return false;
    }
    let mut text = String::from("# kooka-pictura adjustment presets v1\n");
    for (name, bytes) in records {
        text.push_str(name);
        text.push('\t');
        text.push_str(&hex_encode(bytes));
        text.push('\n');
    }
    let tmp = path.with_extension("presets.tmp");
    let write = || -> std::io::Result<()> {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, &path)
    };
    if write().is_ok() {
        return true;
    }
    let _ = fs::remove_file(&tmp);
    // Windows refuses to rename over an existing file; retry after removing it.
    let _ = fs::remove_file(&path);
    write().is_ok()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn hex_decode(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if !text.is_ascii() || !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_adjustment_block;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("pictura-presets-{}-{tag}", std::process::id()))
    }

    fn levels() -> AdjustmentData {
        default_adjustment_block("levels", [0; 3], [255; 3]).unwrap()
    }

    #[test]
    fn preset_store_round_trips() {
        let root = temp_root("round-trip");
        let _ = fs::remove_dir_all(&root);
        let block = levels();
        assert!(save_at(&root, "levels", "My Look", &block));
        assert_eq!(saved_names_at(&root, "levels"), ["My Look"]);
        assert_eq!(
            saved_block_at(&root, "levels", "My Look"),
            Some(block.clone())
        );

        let edited = crate::set_adjustment_param(&block, "inputBlack", 5.0).unwrap();
        assert!(save_at(&root, "levels", "My Look", &edited));
        assert_eq!(saved_names_at(&root, "levels").len(), 1);
        assert_eq!(saved_block_at(&root, "levels", "My Look"), Some(edited));

        assert!(delete_at(&root, "levels", "My Look"));
        assert!(saved_names_at(&root, "levels").is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn preset_store_rejects_reserved_names() {
        let root = temp_root("reserved");
        let _ = fs::remove_dir_all(&root);
        let block = levels();
        for name in ["Default", "Custom", "  ", ""] {
            assert!(!save_at(&root, "levels", name, &block), "{name:?}");
        }
        assert!(saved_names_at(&root, "levels").is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn preset_store_ignores_malformed_lines() {
        let root = temp_root("malformed");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("levels.presets"),
            "# comment\nno tab\nBad Name\tzzzz\nOk\t00\n",
        )
        .unwrap();
        assert!(saved_names_at(&root, "levels").is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn preset_kind_mapping_covers_the_catalogue() {
        for kind in [
            "levels",
            "curves",
            "exposure",
            "hue-saturation",
            "black-white",
            "channel-mixer",
            "photo-filter",
            "selective-color",
            "vibrance",
        ] {
            let block = default_adjustment_block(kind, [0; 3], [255; 3])
                .unwrap_or_else(|| panic!("{kind} has a default"));
            assert_eq!(kind_of(&block), Some(kind), "{kind}");
        }
    }

    #[test]
    fn decoded_blocks_match() {
        let a = levels();
        let b = levels();
        assert!(blocks_match(&a, &b));
        let c = crate::set_adjustment_param(&a, "inputBlack", 1.0).unwrap();
        assert!(!blocks_match(&a, &c));
    }

    #[test]
    fn saveable_and_menu_kinds_match_the_spec() {
        assert_eq!(SAVEABLE_KINDS.len(), 7);
        assert_eq!(MENU_KINDS.len(), 5);
        for kind in SAVEABLE_KINDS {
            assert!(is_saveable(kind), "{kind}");
        }
        for kind in ["photo-filter", "vibrance", "brightness-contrast", "invert"] {
            assert!(!is_saveable(kind), "{kind}");
        }
        for kind in MENU_KINDS {
            assert!(supports_menu(kind), "{kind}");
        }
        assert!(!supports_menu("brightness-contrast"));
    }
}
