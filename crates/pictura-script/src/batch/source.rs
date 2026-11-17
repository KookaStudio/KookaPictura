//! Folder source resolution for Batch.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The openable files in `root` (and, with `recurse`, its non-hidden
/// subfolders, following folder links once each), ordered by their path
/// relative to `root` compared case-insensitively. `extensions` are lowercase
/// suffixes without the dot. Hidden entries (a leading `.`) are skipped.
pub fn resolve_folder(root: &Path, recurse: bool, extensions: &[String]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut visited = HashSet::new();
    walk(root, "", recurse, extensions, &mut visited, &mut found);
    found.sort_by(|a: &(String, PathBuf), b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    found.into_iter().map(|(_, path)| path).collect()
}

fn walk(
    dir: &Path,
    prefix: &str,
    recurse: bool,
    extensions: &[String],
    visited: &mut HashSet<PathBuf>,
    found: &mut Vec<(String, PathBuf)>,
) {
    let Ok(canonical) = fs::canonicalize(dir) else {
        return;
    };
    if !visited.insert(canonical) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        // `fs::metadata` follows links, so an aliased folder is walked.
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        let relative = format!("{prefix}{name}");
        if meta.is_dir() {
            if recurse {
                walk(
                    &path,
                    &format!("{relative}/"),
                    recurse,
                    extensions,
                    visited,
                    found,
                );
            }
        } else if meta.is_file() && has_extension(&name, extensions) {
            found.push((relative.to_lowercase(), path));
        }
    }
}

fn has_extension(name: &str, extensions: &[String]) -> bool {
    name.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty() && extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    })
}
