//! Pins the build-configuration invariants the CMake and Cargo setup relies on
//! but cannot check itself: the bridge-stamp shortcut in `CMakeLists.txt`, the
//! `dev-app` profile's runtime semantics, and the disabled doctests.

use std::fs;
use std::path::{Path, PathBuf};

// Split so this file does not itself read as a bridge file to the CMake scan.
const BRIDGE_MARKER: &str = concat!("cxx_qt", "::bridge");
const BRIDGE_ATTRIBUTE: &str = concat!("#[cxx_qt", "::bridge");

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Byte offset of the `}` that closes the first `{` at or after `from`,
/// skipping comments and string, raw-string, byte-string and char literals.
fn brace_counted_end(src: &str, from: usize) -> Option<usize> {
    let b = src.as_bytes();
    let mut i = from;
    let mut depth = 0usize;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let mut nest = 1;
                i += 2;
                while i < b.len() && nest > 0 {
                    if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                        nest += 1;
                        i += 1;
                    } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                        nest -= 1;
                        i += 1;
                    }
                    i += 1;
                }
                continue;
            }
            b'r' if is_raw_string_start(b, i) => {
                let mut j = i + 1;
                let mut hashes = 0;
                while b[j] == b'#' {
                    hashes += 1;
                    j += 1;
                }
                j += 1;
                let close: Vec<u8> = std::iter::once(b'"')
                    .chain(std::iter::repeat_n(b'#', hashes))
                    .collect();
                while j < b.len() && !b[j..].starts_with(&close) {
                    j += 1;
                }
                i = j + close.len();
                continue;
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'\'' => {
                // A char literal ('x', '\n', '\u{..}', '{'), not a lifetime.
                if b.get(i + 1) == Some(&b'\\') {
                    // Skip the backslash and the escaped character, so `'\''`
                    // does not stop at its escaped quote.
                    i += 3;
                    while i < b.len() && b[i] != b'\'' {
                        i += 1;
                    }
                } else if let Some(c) = src[i + 1..].chars().next() {
                    let after = i + 1 + c.len_utf8();
                    if b.get(after) == Some(&b'\'') {
                        i = after;
                    }
                }
            }
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn is_raw_string_start(b: &[u8], i: usize) -> bool {
    let ident_before = i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
    if ident_before && !(b[i - 1] == b'b' && (i < 2 || !b[i - 2].is_ascii_alphanumeric())) {
        return false;
    }
    let mut j = i + 1;
    while b.get(j) == Some(&b'#') {
        j += 1;
    }
    b.get(j) == Some(&b'"')
}

/// The CMake bridge stamp hashes each bridge module from its attribute to the
/// first column-0 `}`. This holds that shortcut to the real end of the module.
/// It also rejects any outer attribute or outer doc comment above the bridge
/// attribute, looking past blank lines and plain `//` comments: the stamp's
/// walk-back hashes only contiguous `#[` and `///` lines, so one it cannot see
/// would change the generated header without changing the stamp. Inner
/// attributes and `//!` lines belong to the file and are allowed.
#[test]
fn bridge_modules_end_at_column_zero() {
    let mut files = Vec::new();
    rust_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let mut bridges = 0;
    for path in files {
        let src = read(&path);
        if !src.contains(BRIDGE_MARKER) {
            continue;
        }
        bridges += 1;
        let name = path.display();
        assert!(!src.contains('\r'), "{name}: CRLF line endings");
        assert_eq!(
            src.matches(BRIDGE_ATTRIBUTE).count(),
            1,
            "{name}: expected exactly one bridge attribute"
        );
        let attr = src.find(BRIDGE_ATTRIBUTE).unwrap();
        assert!(
            attr == 0 || src.as_bytes()[attr - 1] == b'\n',
            "{name}: the bridge attribute is not at column 0"
        );

        let column_zero_end = src[attr..]
            .find("\n}\n")
            .map(|at| attr + at + 1)
            .unwrap_or_else(|| panic!("{name}: no column-0 `}}` after the bridge attribute"));
        let counted_end = brace_counted_end(&src, attr)
            .unwrap_or_else(|| panic!("{name}: the bridge module never closes"));
        assert_eq!(
            column_zero_end, counted_end,
            "{name}: the first column-0 `}}` is not the end of the bridge module"
        );

        let above = src[..attr].lines().rev().map(str::trim).find(|l| {
            let plain_comment =
                l.starts_with("//") && !l.starts_with("///") && !l.starts_with("//!");
            !l.is_empty() && !plain_comment
        });
        if let Some(line) = above {
            let inner = line.starts_with("#![") || line.starts_with("//!");
            let attribute_like = !inner
                && (line.starts_with("#[")
                    || line.starts_with("///")
                    || line.starts_with("/**")
                    || line.ends_with(']')
                    || line.ends_with("*/"));
            assert!(
                !attribute_like,
                "{name}: `{line}` above the bridge attribute is attached to the module, \
                 and the bridge stamp's walk-back may not hash it"
            );
        }
    }
    assert!(bridges > 0, "no bridge files found");
}

/// Lines of every `[profile.dev-app...]` section, keyed by section header.
fn dev_app_sections(manifest: &str) -> Vec<(String, Vec<String>)> {
    let mut sections = Vec::new();
    let mut current: Option<(String, Vec<String>)> = None;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            sections.extend(current.take());
            if line.starts_with("[profile.dev-app") {
                current = Some((line.to_string(), Vec::new()));
            }
        } else if let Some((_, body)) = current.as_mut() {
            if !line.is_empty() && !line.starts_with('#') {
                body.push(line.to_string());
            }
        }
    }
    sections.extend(current);
    sections
}

/// The development profile must not change runtime behaviour: a panic in a
/// cxx-qt call aborts the process, so `dev`-style overflow checks would crash
/// the app where the shipped build wraps.
#[test]
fn dev_app_profile_keeps_release_semantics() {
    let manifest = read(&repo_root().join("Cargo.toml"));
    let sections = dev_app_sections(&manifest);
    let base = sections
        .iter()
        .find(|(header, _)| header == "[profile.dev-app]")
        .expect("Cargo.toml has no [profile.dev-app]");
    assert!(
        base.1
            .iter()
            .any(|l| l.replace(' ', "") == "inherits=\"release\""),
        "[profile.dev-app] must inherit release"
    );
    for (header, body) in &sections {
        for line in body {
            let key = line.split('=').next().unwrap_or("").trim();
            assert!(
                key != "overflow-checks" && key != "debug-assertions",
                "{header} sets {key}, which changes runtime semantics"
            );
        }
    }
}

fn workspace_members(manifest: &str) -> Vec<String> {
    let start = manifest
        .find("\nmembers = [")
        .expect("no workspace members");
    let list = &manifest[start..];
    let list = &list[..list.find(']').unwrap()];
    list.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// No doc example may compile into a test that no gate runs.
#[test]
fn every_workspace_library_disables_doctests() {
    let root = repo_root();
    let members = workspace_members(&read(&root.join("Cargo.toml")));
    assert!(!members.is_empty());
    for member in members {
        let dir = root.join(&member);
        let manifest = read(&dir.join("Cargo.toml"));
        let has_lib = dir.join("src/lib.rs").exists() || manifest.contains("\n[lib]");
        if !has_lib {
            continue;
        }
        let lib = manifest
            .split("\n[lib]")
            .nth(1)
            .unwrap_or_else(|| panic!("{member}: no [lib] section"));
        let lib = lib.split("\n[").next().unwrap();
        assert!(
            lib.lines().any(|l| l.replace(' ', "") == "doctest=false"),
            "{member}: [lib] does not set doctest = false"
        );
    }
    for gate in [".github/workflows/ci.yml", "scripts/test-report.sh"] {
        for line in read(&root.join(gate)).lines() {
            assert!(
                !(line.contains("cargo test") && line.contains("--doc")),
                "{gate} still runs doctests: {line}"
            );
        }
    }
}
