//! The checked-in verb manifest: every verb a bridge module declares, with the
//! `cpp/api/` header that owns it, so a C++ caller can find the header to
//! include without reading a generated header or grepping the bridges.
//!
//! The manifest is generated, never hand-edited. Run the manifest tests with
//! `PICTURA_WRITE_VERB_MANIFEST=1` to rewrite `verb_manifest.tsv`.

use std::collections::HashSet;
use std::path::Path;

use super::{bridge_files, bridge_header_owners, bridge_module, generated_header, read};

/// The manifest, next to this module so `include_str!` reaches it.
const MANIFEST: &str = include_str!("verb_manifest.tsv");

/// Rewrites the manifest in place when set; a normal test run only compares.
const WRITE_ENV: &str = "PICTURA_WRITE_VERB_MANIFEST";

const HEADER: &str = "kind\tcpp_name\trust_name\towner\tbridge";

const KINDS: [&str; 3] = ["free", "qinvokable", "qsignal"];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Verb {
    kind: &'static str,
    cpp_name: String,
    rust_name: String,
    owner: String,
    bridge: String,
}

impl Verb {
    fn row(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}",
            self.kind, self.cpp_name, self.rust_name, self.owner, self.bridge
        )
    }

    fn order(&self) -> (&str, &str, &str, &str) {
        (
            self.owner.as_str(),
            self.kind,
            &self.cpp_name,
            &self.rust_name,
        )
    }
}

/// The `#[...]` attributes directly above a declaration, nearest first.
///
/// A declaration's attributes sit immediately above it, interleaved with its doc
/// comments (`/// doc`, `#[cxx_name]`, `/// doc`, `#[qinvokable]`, `fn`), so the
/// walk passes over both. Blank lines and the declaration above break it:
/// continuing past one would collect the previous declaration's attributes.
fn attributes_above(lines: &[&str], index: usize) -> String {
    let mut found = Vec::new();
    let mut at = index;
    while at > 0 {
        at -= 1;
        let line = lines[at].trim();
        if line.starts_with("///") {
            continue;
        }
        if !line.starts_with("#[") {
            break;
        }
        found.push(line);
    }
    found.join("\n")
}

fn attribute_value<'a>(attributes: &'a str, name: &str) -> Option<&'a str> {
    for line in attributes.lines() {
        let Some(inner) = line
            .strip_prefix("#[")
            .and_then(|inner| inner.strip_suffix(']'))
        else {
            continue;
        };
        let Some(rest) = inner.trim_start().strip_prefix(name) else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('"') else {
            continue;
        };
        let end = rest.find('"')?;
        return Some(&rest[..end]);
    }
    None
}

/// Every verb declared in `module`'s `extern "Rust"`/`extern "RustQt"` blocks.
/// `extern "C++"` blocks are skipped: they declare functions C++ provides, not
/// verbs Rust exposes.
fn verbs_in_module(module: &str) -> Vec<(&'static str, String, String)> {
    let mut verbs = Vec::new();
    let mut rest = module;
    while let Some(at) = rest.find("extern \"Rust") {
        let block = &rest[at..];
        let Some(open) = block.find('{') else {
            break;
        };
        let Some(close) = super::brace_counted_end(block, open) else {
            break;
        };
        let body = &block[open + 1..close];
        let lines: Vec<&str> = body.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let Some(declaration) = trimmed.strip_prefix("fn ") else {
                continue;
            };
            let rust_name: String = declaration
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if rust_name.is_empty() {
                continue;
            }
            let attributes = attributes_above(&lines, index);
            let kind = if attributes.contains("#[qsignal]") {
                "qsignal"
            } else if attributes.contains("#[qinvokable]") {
                "qinvokable"
            } else {
                "free"
            };
            let cpp_name = attribute_value(&attributes, "cxx_name")
                .unwrap_or(&rust_name)
                .to_string();
            verbs.push((kind, cpp_name, rust_name));
        }
        rest = &block[close + 1..];
    }
    verbs
}

/// Every declared verb in the tree, attributed to its owner and bridge header.
fn collected_verbs() -> Vec<Verb> {
    let owners = bridge_header_owners();
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut verbs = Vec::new();
    for path in bridge_files() {
        let src = read(&path);
        let Some(module) = bridge_module(&src) else {
            panic!(
                "{}: carries the bridge marker but no bridge module",
                path.display()
            );
        };
        let declared = verbs_in_module(module);
        if declared.is_empty() {
            // Import-only bridges (extern "C++" only) expose no verb and need no
            // owner.
            continue;
        }
        let relative = path.strip_prefix(manifest_dir.join("src")).unwrap();
        let bridge = generated_header(relative);
        let owner = owners.get(&bridge).unwrap_or_else(|| {
            panic!(
                "{bridge} is declared by {} but no cpp/api file includes it",
                path.display()
            )
        });
        for (kind, cpp_name, rust_name) in declared {
            verbs.push(Verb {
                kind,
                cpp_name,
                rust_name,
                owner: owner.clone(),
                bridge: bridge.clone(),
            });
        }
    }
    verbs.sort_by(|a, b| a.order().cmp(&b.order()));
    verbs
}

fn rendered(verbs: &[Verb]) -> String {
    let mut out = String::new();
    out.push_str("# Generated by the verb_manifest tests; do not edit.\n");
    out.push_str("# Regenerate: PICTURA_WRITE_VERB_MANIFEST=1 cargo nextest run -p pictura-app verb_manifest\n");
    for comment in [
        "# Columns: kind, the name C++ sees (#[cxx_name] when declared), the Rust name,",
        "# the cpp/api/ header that owns the bridge's generated header, the bridge header.",
    ] {
        out.push_str(comment);
        out.push('\n');
    }
    out.push_str(HEADER);
    out.push('\n');
    for verb in verbs {
        out.push_str(&verb.row());
        out.push('\n');
    }
    out
}

/// Rewrite the checked-in manifest when asked; returns whether it did.
fn bless(verbs: &[Verb]) -> bool {
    if std::env::var(WRITE_ENV).as_deref() != Ok("1") {
        return false;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/build_config/verb_manifest.tsv");
    std::fs::write(&path, rendered(verbs))
        .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    let written =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let expected: Vec<String> = verbs.iter().map(Verb::row).collect();
    assert_eq!(
        parse_rows(&written),
        expected,
        "verb_manifest.tsv did not round-trip"
    );
    true
}

fn parse_rows(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty() && *line != HEADER)
        .map(str::to_owned)
        .collect()
}

/// The manifest rows, in file order, without the comment lines or header.
fn manifest_rows() -> Vec<String> {
    parse_rows(MANIFEST)
}

#[test]
fn verb_manifest_matches_bridge_sources() {
    let verbs = collected_verbs();
    if bless(&verbs) {
        return;
    }
    let expected: Vec<String> = verbs.iter().map(Verb::row).collect();
    let actual = manifest_rows();
    let missing: Vec<&String> = expected
        .iter()
        .filter(|row| !actual.contains(row))
        .collect();
    let extra: Vec<&String> = actual
        .iter()
        .filter(|row| !expected.contains(row))
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "verb_manifest.tsv differs from the bridge sources\n\
         missing ({}): {missing:#?}\nextra ({}): {extra:#?}\n\
         regenerate with {WRITE_ENV}=1",
        missing.len(),
        extra.len()
    );
    assert_eq!(actual, expected, "the manifest rows are out of order");
}

#[test]
fn verb_manifest_counts_every_declared_function() {
    let verbs = collected_verbs();
    let modules: usize = bridge_files()
        .iter()
        .map(|path| {
            let src = read(path);
            super::rust_function_count(bridge_module(&src).expect("a bridge module"))
        })
        .sum();
    assert_eq!(
        verbs.len(),
        modules,
        "the manifest rows and the shipped function counter disagree"
    );
    assert!(!verbs.is_empty(), "no verbs collected");
}

#[test]
fn verb_manifest_names_owners() {
    let verbs = collected_verbs();
    for verb in &verbs {
        assert!(
            verb.owner.starts_with("api/") && !verb.owner.ends_with('/'),
            "{} has no cpp/api owner",
            verb.rust_name
        );
    }
    let header_owners = bridge_header_owners();
    let mut owners: HashSet<String> = HashSet::new();
    let mut declared: HashSet<String> = HashSet::new();
    for verb in &verbs {
        owners.insert(verb.owner.clone());
    }
    for owner in header_owners.values() {
        declared.insert(owner.clone());
    }
    assert_eq!(
        owners, declared,
        "every adopting header should own at least one verb"
    );
}

#[test]
fn verb_manifest_names_renamed_verbs() {
    let verbs = collected_verbs();
    let renamed: Vec<&Verb> = verbs
        .iter()
        .filter(|verb| verb.cpp_name != verb.rust_name)
        .collect();
    assert!(!renamed.is_empty(), "no #[cxx_name] verb found");
    for verb in renamed {
        assert!(
            KINDS.contains(&verb.kind),
            "{} has an unknown kind",
            verb.rust_name
        );
        assert!(
            !verb.cpp_name.is_empty(),
            "{} lost its C++ name",
            verb.rust_name
        );
    }
}
