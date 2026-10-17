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
        let mut command = Command::new(&self.exe);
        command
            .args(["-dSAFER", "-dBATCH", "-dNOPAUSE", "-dNOPROMPT"])
            .arg(format!("--permit-file-read={}", input.display()));
        if let Some(password) = password.filter(|p| !p.is_empty()) {
            command.arg(format!("-sPDFPassword={password}"));
        }
        command.args(args);
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
    pub(crate) fn new() -> Result<ScratchDir, RasterError> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "pictura-gs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).map_err(|e| RasterError::Failed(e.to_string()))?;
        Ok(ScratchDir(dir))
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
