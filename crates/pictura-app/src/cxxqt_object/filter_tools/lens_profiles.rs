//! The installed lens profiles (#932): the bundled set, then the lensfun
//! database the system or the user installed (never bundled: it is CC BY-SA
//! 3.0), then the user's in-tree files, loaded once per process. When two
//! share an id, the later source wins.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pictura_filters::{builtin_lens_profiles, parse_lens_profiles, parse_lensfun, LensProfile};

/// Every installed profile.
pub(super) fn installed() -> &'static [LensProfile] {
    static PROFILES: OnceLock<Vec<LensProfile>> = OnceLock::new();
    PROFILES.get_or_init(|| load(&|key| std::env::var(key).ok()))
}

/// The installed profile with `id`.
pub(super) fn by_id(id: i32) -> Option<&'static LensProfile> {
    installed().iter().find(|p| p.id() == id)
}

type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// `$XDG_DATA_HOME`, else `~/.local/share`.
fn data_home(env: Env) -> Option<PathBuf> {
    env("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| env("HOME").map(|h| Path::new(&h).join(".local/share")))
}

/// lensfun's roots, lowest priority first, without repeats (a repeat keeps
/// its later place). `PICTURA_LENSFUN_DIR`, when set, is the only root.
fn lensfun_roots(env: Env) -> Vec<PathBuf> {
    if let Some(dir) = env("PICTURA_LENSFUN_DIR").filter(|v| !v.is_empty()) {
        return vec![PathBuf::from(dir)];
    }
    let mut roots = vec![
        PathBuf::from("/usr/share/lensfun"),
        PathBuf::from("/usr/local/share/lensfun"),
    ];
    let dirs = env("XDG_DATA_DIRS")
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    // The first XDG_DATA_DIRS entry is the most important.
    roots.extend(
        dirs.split(':')
            .filter(|d| !d.is_empty())
            .rev()
            .map(|d| Path::new(d).join("lensfun")),
    );
    roots.push(PathBuf::from("/var/lib/lensfun-updates"));
    if let Some(home) = data_home(env) {
        roots.push(home.join("lensfun"));
        roots.push(home.join("lensfun/updates"));
    }
    let mut seen = HashSet::new();
    let mut out: Vec<PathBuf> = roots
        .into_iter()
        .rev()
        .filter(|r| seen.insert(r.clone()))
        .collect();
    out.reverse();
    out
}

/// The `*.<ext>` files directly in `dir`, sorted.
fn files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == ext))
        .collect();
    out.sort();
    out
}

/// The lensfun files to read: each root's `version_1/`, `version_2/`, and
/// the root itself, a file name found again later replacing the earlier
/// file, as lensfun does.
fn lensfun_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut by_name: BTreeMap<std::ffi::OsString, PathBuf> = BTreeMap::new();
    for root in roots {
        for dir in [root.join("version_1"), root.join("version_2"), root.clone()] {
            for path in files(&dir, "xml") {
                if let Some(name) = path.file_name() {
                    by_name.insert(name.to_owned(), path);
                }
            }
        }
    }
    by_name.into_values().collect()
}

/// The profiles in `paths`; a file that cannot be read or parsed is skipped
/// with a warning.
fn read_all(
    paths: &[PathBuf],
    parse: fn(&str) -> Result<Vec<LensProfile>, String>,
) -> Vec<LensProfile> {
    let mut out = Vec::new();
    for path in paths {
        match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| parse(&text))
        {
            Ok(profiles) => out.extend(profiles),
            Err(e) => eprintln!("pictura: skipping lens profiles {}: {e}", path.display()),
        }
    }
    out
}

fn load(env: Env) -> Vec<LensProfile> {
    let mut all = builtin_lens_profiles().to_vec();
    all.extend(read_all(&lensfun_files(&lensfun_roots(env)), parse_lensfun));
    if let Some(home) = data_home(env) {
        let user = files(&home.join("pictura/lens-profiles"), "txt");
        all.extend(read_all(&user, parse_lens_profiles));
    }
    let mut seen = HashSet::new();
    let mut out: Vec<LensProfile> = all
        .into_iter()
        .rev()
        .filter(|p| seen.insert(p.id()))
        .collect();
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    /// A fresh directory under the system temp dir.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pictura-lens-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// A synthetic lensfun file holding one lens with an invented poly5 entry.
    fn lens_xml(model: &str) -> String {
        format!(
            "<lensdatabase><lens><maker>ACME</maker><model>{model}</model>\
             <cropfactor>1</cropfactor><calibration>\
             <distortion model=\"poly5\" focal=\"50\" k1=\"-0.01\" k2=\"0\"/>\
             </calibration></lens></lensdatabase>"
        )
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(path, text).expect("write");
    }

    #[test]
    fn lensfun_dirs_follow_xdg_and_the_override() {
        let roots = lensfun_roots(&env(&[
            ("XDG_DATA_DIRS", "/opt/a:/opt/b"),
            ("XDG_DATA_HOME", "/home/u/.data"),
        ]));
        let expect: Vec<PathBuf> = [
            "/usr/share/lensfun",
            "/usr/local/share/lensfun",
            "/opt/b/lensfun",
            "/opt/a/lensfun",
            "/var/lib/lensfun-updates",
            "/home/u/.data/lensfun",
            "/home/u/.data/lensfun/updates",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        assert_eq!(roots, expect);
        // The default XDG_DATA_DIRS repeats the system roots; HOME stands in
        // for an unset XDG_DATA_HOME.
        let roots = lensfun_roots(&env(&[("HOME", "/home/u")]));
        assert_eq!(roots[0], PathBuf::from("/usr/share/lensfun"));
        assert_eq!(roots[1], PathBuf::from("/usr/local/share/lensfun"));
        assert_eq!(roots[2], PathBuf::from("/var/lib/lensfun-updates"));
        assert_eq!(roots[3], PathBuf::from("/home/u/.local/share/lensfun"));
        assert_eq!(roots.len(), 5);
        assert_eq!(
            lensfun_roots(&env(&[
                ("PICTURA_LENSFUN_DIR", "/t/db"),
                ("HOME", "/home/u")
            ])),
            vec![PathBuf::from("/t/db")]
        );
    }

    #[test]
    fn later_files_replace_earlier_ones() {
        let dir = scratch("replace");
        let (low, high) = (dir.join("low"), dir.join("high"));
        write(
            &low.join("version_1/slr-acme.xml"),
            &lens_xml("ACME Old 50mm"),
        );
        write(
            &low.join("version_1/only-low.xml"),
            &lens_xml("ACME Low 35mm"),
        );
        write(&high.join("slr-acme.xml"), &lens_xml("ACME New 50mm"));
        write(&high.join("notes.txt"), "not a database");
        let paths = lensfun_files(&[low.clone(), high.clone()]);
        assert_eq!(
            paths,
            vec![
                low.join("version_1/only-low.xml"),
                high.join("slr-acme.xml")
            ]
        );
        let names: Vec<String> = read_all(&paths, parse_lensfun)
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["ACME Low 35mm", "ACME New 50mm"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn user_profiles_load_from_the_data_home() {
        let dir = scratch("user");
        let db = dir.join("db");
        write(&db.join("version_1/acme.xml"), &lens_xml("ACME 85mm"));
        write(&db.join("version_1/broken.xml"), "<lensdatabase><lens>");
        let home = dir.join("data");
        write(
            &home.join("pictura/lens-profiles/mine.txt"),
            "[profile]\nname = Mine\nmake = ACME Cameras\nraw = yes\n",
        );
        write(&home.join("pictura/lens-profiles/bad.txt"), "zoom = 1\n");
        // A copy of a profile (same id) is installed once.
        write(
            &home.join("pictura/lens-profiles/copy.txt"),
            "[profile]\nname = Mine\nmake = ACME Cameras\nraw = yes\n",
        );
        let (db_s, home_s) = (db.display().to_string(), home.display().to_string());
        let all = load(&env(&[
            ("PICTURA_LENSFUN_DIR", db_s.as_str()),
            ("XDG_DATA_HOME", home_s.as_str()),
        ]));
        let names: Vec<&str> = all.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names.len(), 3, "{names:?}");
        assert_eq!(names[0], builtin_lens_profiles()[0].name);
        assert_eq!(names[1..], ["ACME 85mm", "Mine"]);
        assert!(all[2].raw);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
