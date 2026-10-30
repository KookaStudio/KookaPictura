//! Pins the build-configuration invariants the CMake and Cargo setup relies on
//! but cannot check itself: the bridge-stamp shortcut in `CMakeLists.txt`, the
//! declarations-only bridge files, the `dev-app` profile's runtime semantics,
//! the disabled doctests, and the justified fixed waits in the Qt Test suites.

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

/// 1-based line of a byte offset.
fn line_of(src: &str, at: usize) -> usize {
    src[..at].matches('\n').count() + 1
}

/// Offset just past the `]` that closes the attribute starting at `from`,
/// skipping string literals such as `#[doc = "]"]`.
fn attribute_end(b: &[u8], from: usize) -> usize {
    let mut depth = 0usize;
    let mut i = from;
    while i < b.len() {
        match b[i] {
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// The top-level items in `src[from..to]` that a bridge file may not hold:
/// anything but a body-less `mod`, a `use`, or an `impl` (or `unsafe impl`)
/// with an empty body.
/// Comments, attributes, and doc comments are skipped. Returns their lines.
fn implementation_items(src: &str, from: usize, to: usize) -> Vec<(usize, String)> {
    let b = src.as_bytes();
    let mut found = Vec::new();
    let mut i = from;
    while i < to {
        if b[i].is_ascii_whitespace() {
            i += 1;
        } else if src[i..].starts_with("//") {
            i += src[i..to].find('\n').unwrap_or(to - i);
        } else if src[i..].starts_with("/*") {
            i += src[i..to].find("*/").map_or(to - i, |end| end + 2);
        } else if b[i] == b'#' {
            i = attribute_end(b, i);
        } else {
            let mut item_start = i;
            if src[i..].starts_with("pub(") {
                item_start = src[i..to].find(')').map_or(to, |close| i + close + 1);
            } else if src[i..].starts_with("pub ") {
                item_start = i + "pub ".len();
            }
            let item_start = (item_start..to)
                .find(|&j| !b[j].is_ascii_whitespace())
                .unwrap_or(to);
            // A `use` may group its paths in braces; it still ends at its `;`.
            let is_use = src[item_start..].starts_with("use ");
            let head_end = (i..to)
                .find(|&j| b[j] == b';' || (!is_use && b[j] == b'{'))
                .unwrap_or(to);
            let head = src[i..head_end]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let item = &src[item_start..head_end];
            let end = if head_end < to && b[head_end] == b'{' {
                brace_counted_end(src, head_end).map_or(to, |end| end + 1)
            } else {
                head_end + 1
            };
            let allowed = if head_end < to && b[head_end] == b'{' {
                (item.starts_with("impl") || item.starts_with("unsafe impl"))
                    && src[head_end + 1..end - 1].trim().is_empty()
            } else {
                item.starts_with("mod ") || item.starts_with("use ")
            };
            if !allowed {
                found.push((line_of(src, i), head));
            }
            i = end;
        }
    }
    found
}

/// The files build.rs hands to cxx-qt: the string literals of its
/// `.files([...])` list, relative to the crate root.
fn listed_bridge_files() -> Vec<String> {
    let build = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"));
    let open = build
        .find(".files([")
        .expect("build.rs has no `.files([` list");
    let start = open + ".files([".len();
    let end = start
        + build[start..]
            .find("])")
            .expect("the `.files([` list never closes");
    build[start..end]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// A bridge file is regenerated and recompiled by the cxx-qt build script on
/// any edit, so it holds the bridge module and the declarations that bind it,
/// and the implementation lives in modules the build script does not track.
#[test]
fn bridge_files_hold_declarations_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = listed_bridge_files();
    assert!(!files.is_empty(), "build.rs lists no bridge files");
    let mut violations = Vec::new();
    for file in files {
        let src = read(&root.join(&file));
        let Some(attr) = src.find(BRIDGE_ATTRIBUTE) else {
            violations.push(format!("{file}: listed in build.rs but holds no bridge"));
            continue;
        };
        let end = brace_counted_end(&src, attr).map_or(src.len(), |end| end + 1);
        for (line, head) in implementation_items(&src, 0, attr)
            .into_iter()
            .chain(implementation_items(&src, end, src.len()))
        {
            violations.push(format!("{file}:{line}: {head}"));
        }
    }
    assert!(
        violations.is_empty(),
        "bridge files hold implementation outside the bridge module:\n{}",
        violations.join("\n")
    );
}

/// A fixed wait in a Qt Test suite either flushes queued events (`qWait(0)`)
/// or says why no condition can replace it, in a comment that trails the call
/// or on a comment-only line right above it.
#[test]
fn qt_test_waits_are_justified() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("cpp/tests");
    let mut sources: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "cpp" || e == "h"))
        .collect();
    sources.sort();
    let mut unjustified = Vec::new();
    for path in sources {
        let src = read(&path);
        let lines: Vec<&str> = src.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let mut rest = *line;
            while let Some(at) = rest.find("qWait(") {
                let args = &rest[at + "qWait(".len()..];
                let mut depth = 1usize;
                let close = args
                    .char_indices()
                    .find(|&(_, c)| {
                        match c {
                            '(' => depth += 1,
                            ')' => depth -= 1,
                            _ => {}
                        }
                        depth == 0
                    })
                    .map_or(args.len(), |(j, _)| j);
                let trailing = args[close..].contains("//");
                let above = index > 0 && lines[index - 1].trim_start().starts_with("//");
                if args[..close].trim() != "0" && !trailing && !above {
                    unjustified.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
                }
                rest = &args[close..];
            }
        }
    }
    assert!(
        unjustified.is_empty(),
        "fixed waits without a reason comment:\n{}",
        unjustified.join("\n")
    );
}
