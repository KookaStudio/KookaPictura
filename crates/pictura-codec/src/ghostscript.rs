//! Ghostscript as an external rasterizer (design D1 of `pdf-eps-import`).
//!
//! `gs` runs as a child process, never linked, with `-dSAFER` and file reads
//! limited to the input. `PICTURA_GHOSTSCRIPT` names the executable; otherwise
//! `gs` is looked up on `PATH`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

/// Why a PDF or EPS could not be rasterized.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RasterError {
    #[error("Ghostscript is required to open PDF and EPS files, but it was not found")]
    NoGhostscript,
    #[error("this PDF requires a password")]
    PasswordRequired,
    #[error("the password did not open this PDF")]
    WrongPassword,
    #[error("Ghostscript could not read the file: {0}")]
    Failed(String),
}

/// An 8-bit RGBA raster, row-major, four bytes per pixel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// One finished Ghostscript run: its text output (stdout, then stderr) and
/// the recoverable problems it reported (`****` lines).
pub(crate) struct Run {
    pub output: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Ghostscript {
    exe: PathBuf,
}

impl Ghostscript {
    /// The Ghostscript to run, or `NoGhostscript` when none is installed.
    pub fn locate() -> Result<Ghostscript, RasterError> {
        let exe = match std::env::var_os("PICTURA_GHOSTSCRIPT") {
            Some(exe) => PathBuf::from(exe),
            None => std::env::var_os("PATH")
                .iter()
                .flat_map(std::env::split_paths)
                .map(|dir| dir.join("gs"))
                .find(|candidate| candidate.is_file())
                .ok_or(RasterError::NoGhostscript)?,
        };
        if exe.is_file() {
            Ok(Ghostscript { exe })
        } else {
            Err(RasterError::NoGhostscript)
        }
    }

    /// Run Ghostscript with `args`, allowed to read only `input`. `password`
    /// opens an encrypted PDF.
    pub(crate) fn run(
        &self,
        input: &Path,
        password: Option<&str>,
        args: &[String],
    ) -> Result<Run, RasterError> {
        // Holds the password argument file until Ghostscript has exited.
        let scratch = ScratchDir::new()?;
        let mut command = self.command(input, password, args, &scratch)?;
        let output = command.output().map_err(|_| RasterError::NoGhostscript)?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let all = format!("{stdout}\n{stderr}");
        if all.contains("Password did not work") {
            return Err(RasterError::WrongPassword);
        }
        if all.contains("requires a password") {
            return Err(RasterError::PasswordRequired);
        }
        if !output.status.success() {
            let reason = all
                .lines()
                .find(|line| line.trim_start().starts_with("Error:"))
                .unwrap_or("unrecoverable error")
                .trim()
                .to_string();
            return Err(RasterError::Failed(reason));
        }
        Ok(Run {
            warnings: warnings(&all),
            output: all,
        })
    }
}

impl Ghostscript {
    /// The Ghostscript command line. A password never appears in argv (where
    /// `ps` and `/proc` would show it): it goes in an owner-only argument file
    /// in `scratch`, passed as `@file`.
    fn command(
        &self,
        input: &Path,
        password: Option<&str>,
        args: &[String],
        scratch: &ScratchDir,
    ) -> Result<Command, RasterError> {
        let mut command = Command::new(&self.exe);
        command
            .args(["-dSAFER", "-dBATCH", "-dNOPAUSE", "-dNOPROMPT"])
            .arg(format!("--permit-file-read={}", input.display()));
        if let Some(password) = password.filter(|p| !p.is_empty()) {
            match password_argument(password) {
                Some(line) => {
                    let file = scratch.path("args");
                    write_private(&file, line.as_bytes())?;
                    command.arg(format!("@{}", file.display()));
                }
                // ponytail: a password ending in `\` (or holding a line
                // break) has no `@file` spelling, so it alone still goes on
                // the command line, visible to `ps` while Ghostscript runs.
                None => {
                    command.arg(format!("-sPDFPassword={password}"));
                }
            }
        }
        command.args(args);
        Ok(command)
    }
}

/// `-sPDFPassword=…` as one quoted argument-file line. Ghostscript's `@file`
/// reader keeps a backslash literally except before `"`, where `\"` is a
/// quote; so only quotes are escaped, and a trailing backslash (which would
/// escape the closing quote) or a line break cannot be spelled: `None`.
fn password_argument(password: &str) -> Option<String> {
    if password.ends_with('\\') || password.contains(['\n', '\r']) {
        return None;
    }
    Some(format!(
        "\"-sPDFPassword={}\"\n",
        password.replace('"', "\\\"")
    ))
}

/// Create `path` (which must not exist) readable and writable by its owner
/// only, holding `bytes`.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), RasterError> {
    use std::io::Write as _;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|e| RasterError::Failed(e.to_string()))
}

/// Ghostscript's `****` complaints as sentences, one per report.
fn warnings(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("****") else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }
        // A report continues over several `****` lines until a sentence ends.
        match out.last_mut() {
            Some(last) if !last.ends_with('.') => {
                last.push(' ');
                last.push_str(rest);
            }
            _ => out.push(rest.to_string()),
        }
    }
    out.dedup();
    out
}

/// What a file's leading bytes say it is, for the formats Ghostscript reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorKind {
    Pdf,
    Eps,
}

/// `%PDF-` within the first KiB is PDF (readers allow leading junk); the DOS
/// EPS header or `%!PS` at the start is EPS / PostScript.
pub fn sniff_vector(head: &[u8]) -> Option<VectorKind> {
    if head.starts_with(&crate::eps::DOS_EPS_MAGIC) || head.starts_with(b"%!PS") {
        return Some(VectorKind::Eps);
    }
    let window = &head[..head.len().min(1024)];
    window
        .windows(5)
        .any(|w| w == b"%PDF-")
        .then_some(VectorKind::Pdf)
}

/// A private scratch directory, removed on drop.
pub(crate) struct ScratchDir(PathBuf);

impl ScratchDir {
    /// A new owner-only directory under the temp dir with an unguessable
    /// name. Creation fails rather than reuse a path that already exists, so
    /// another user cannot pre-create (or symlink) it.
    pub(crate) fn new() -> Result<ScratchDir, RasterError> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let mut last_error = None;
        for _ in 0..8 {
            let dir = std::env::temp_dir().join(format!(
                "pictura-gs-{}-{}-{:016x}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
                random_u64()
            ));
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
            match builder.create(&dir) {
                Ok(()) => return Ok(ScratchDir(dir)),
                Err(e) => last_error = Some(e),
            }
        }
        Err(RasterError::Failed(
            last_error.map_or_else(String::new, |e| e.to_string()),
        ))
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 64 unpredictable bits: std's `RandomState` keys are seeded from the OS
/// random source (the `os_entropy` pattern of the PDF writer's bridge).
fn random_u64() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    hasher.write_u128(nanos);
    hasher.finish()
}

/// Read a `pngalpha` page Ghostscript wrote.
pub(crate) fn read_png(path: &Path) -> Result<Raster, RasterError> {
    let bytes = std::fs::read(path).map_err(|e| RasterError::Failed(e.to_string()))?;
    crate::png::decode_rgba(&bytes)
        .ok_or_else(|| RasterError::Failed("unreadable Ghostscript output".into()))
}

/// The `pngalpha` arguments rendering `input` at `rx` × `ry` pixels per inch.
pub(crate) fn png_args(
    rx: f64,
    ry: f64,
    anti_alias: bool,
    out: &Path,
    input: &Path,
) -> Vec<String> {
    let bits = if anti_alias { 4 } else { 1 };
    vec![
        "-sDEVICE=pngalpha".into(),
        format!("-r{rx:.4}x{ry:.4}"),
        format!("-dTextAlphaBits={bits}"),
        format!("-dGraphicsAlphaBits={bits}"),
        format!("-sOutputFile={}", out.display()),
        "-f".into(),
        input.display().to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_recognizes_pdf_and_eps() {
        assert_eq!(sniff_vector(b"%PDF-1.7\n"), Some(VectorKind::Pdf));
        assert_eq!(sniff_vector(b"\x00junk%PDF-1.4"), Some(VectorKind::Pdf));
        assert_eq!(
            sniff_vector(b"%!PS-Adobe-3.0 EPSF-3.0"),
            Some(VectorKind::Eps)
        );
        assert_eq!(
            sniff_vector(&[0xC5, 0xD0, 0xD3, 0xC6, 0, 0]),
            Some(VectorKind::Eps)
        );
        assert_eq!(sniff_vector(b"\x89PNG\r\n"), None);
        let mut late = vec![b' '; 1100];
        late.extend_from_slice(b"%PDF-1.4");
        assert_eq!(sniff_vector(&late), None, "only the first KiB counts");
    }

    #[test]
    fn missing_ghostscript_is_an_error() {
        let found = Ghostscript {
            exe: PathBuf::from("/nonexistent/pictura-gs"),
        };
        let input = Path::new("/nonexistent/input.pdf");
        assert_eq!(
            found.run(input, None, &[]).err(),
            Some(RasterError::NoGhostscript)
        );
    }

    #[test]
    fn password_stays_out_of_argv() {
        let gs = Ghostscript {
            exe: PathBuf::from("/bin/true"),
        };
        let scratch = ScratchDir::new().unwrap();
        let secret = "s3cret \"quoted\" back\\slash";
        let command = gs
            .command(Path::new("/x/in.pdf"), Some(secret), &[], &scratch)
            .unwrap();
        let argv: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(
            argv.iter()
                .all(|a| !a.contains("s3cret") && !a.contains("PDFPassword")),
            "{argv:?}"
        );
        let file = scratch.path("args");
        assert!(argv.contains(&format!("@{}", file.display())));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "\"-sPDFPassword=s3cret \\\"quoted\\\" back\\slash\"\n"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&file).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn unspellable_passwords_have_no_argument_file_line() {
        assert_eq!(password_argument("end\\"), None);
        assert_eq!(password_argument("two\nlines"), None);
        assert_eq!(
            password_argument("a\\\"b").as_deref(),
            Some("\"-sPDFPassword=a\\\\\"b\"\n")
        );
    }

    #[test]
    fn scratch_dirs_are_private_and_unique() {
        let a = ScratchDir::new().unwrap();
        let b = ScratchDir::new().unwrap();
        assert_ne!(a.0, b.0);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&a.0).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
        }
        let gone = a.0.clone();
        drop(a);
        assert!(!gone.exists(), "removed on drop");
    }

    #[test]
    fn warnings_join_continuation_lines() {
        let text = "Page 1\n   **** This file had errors that were repaired or ignored.\n   **** Please notify the author of the software that produced this\n   **** file that it does not conform to Adobe's published PDF\n   **** specification.\n";
        assert_eq!(
            warnings(text),
            vec![
                "This file had errors that were repaired or ignored.".to_string(),
                "Please notify the author of the software that produced this file that it does not conform to Adobe's published PDF specification.".to_string(),
            ]
        );
    }
}
