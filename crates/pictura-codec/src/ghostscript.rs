//! Ghostscript as an external rasterizer (design D1 of `pdf-eps-import`).
//!
//! `gs` runs as a child process, never linked, with `-dSAFER` and file reads
//! limited to the input. `PICTURA_GHOSTSCRIPT` names the executable; otherwise
//! `gs` is looked up on `PATH`.
//!
//! The input is untrusted and Ghostscript is a full PostScript interpreter, so
//! each run is bounded (#844): it is killed after [`RUN_TIMEOUT`], on Unix it
//! runs under address-space, CPU, and file-size rlimits, its text output is
//! capped, and a release older than [`MIN_VERSION`] is refused.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// How long one Ghostscript run may take before it is killed.
pub(crate) const RUN_TIMEOUT: Duration = Duration::from_secs(300);
/// The oldest Ghostscript accepted: 9.55 is the first release with the
/// rewritten `-dSAFER` file-permission model plus the CVE-2021-3781 fix.
pub(crate) const MIN_VERSION: (u32, u32) = (9, 55);
/// The most stdout or stderr text kept from one run.
const MAX_TEXT_OUTPUT: u64 = 16 << 20;
/// The most bytes one rendered page file may hold when it is read back.
const MAX_RASTER_FILE: u64 = 4 << 30;
/// The largest page bitmap Ghostscript holds whole (#646); a larger page is
/// rendered in bands, so its memory follows the band, not the page, and stays
/// under [`MAX_ADDRESS_SPACE`]. An RGBA page about 11 500 px square fits.
const MAX_PAGE_BITMAP: u64 = 512 << 20;
/// The address space one Ghostscript process may map (Unix `RLIMIT_AS`).
#[cfg(unix)]
const MAX_ADDRESS_SPACE: u64 = 8 << 30;

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
        if !exe.is_file() {
            return Err(RasterError::NoGhostscript);
        }
        let found = Ghostscript { exe };
        found.check_version()?;
        Ok(found)
    }

    /// Refuse a Ghostscript older than [`MIN_VERSION`] (or one that cannot
    /// report its version).
    fn check_version(&self) -> Result<(), RasterError> {
        let mut command = Command::new(&self.exe);
        command.arg("--version");
        let output = run_bounded(command, Duration::from_secs(30))?;
        let text = String::from_utf8_lossy(&output.stdout);
        match parse_version(&text) {
            Some(found) if found >= MIN_VERSION => Ok(()),
            _ => Err(RasterError::Failed(format!(
                "Ghostscript {} is too old; {}.{} or later is required",
                text.trim(),
                MIN_VERSION.0,
                MIN_VERSION.1
            ))),
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
        self.run_within(input, password, args, RUN_TIMEOUT)
    }

    fn run_within(
        &self,
        input: &Path,
        password: Option<&str>,
        args: &[String],
        timeout: Duration,
    ) -> Result<Run, RasterError> {
        // Holds the password argument file until Ghostscript has exited.
        let scratch = ScratchDir::new()?;
        let (input, args) = permitted_input(input, args, &scratch)?;
        let command = self.command(&input, password, &args, &scratch)?;
        let output = run_bounded(command, timeout)?;
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

/// The finished output of one bounded child process.
struct Output {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Run `command` with its output captured (each stream capped at
/// [`MAX_TEXT_OUTPUT`]), under the Unix resource limits, killing it after
/// `timeout`. A command that cannot start is `NoGhostscript`.
fn run_bounded(mut command: Command, timeout: Duration) -> Result<Output, RasterError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    limit_resources(&mut command, timeout);
    let mut child = command.spawn().map_err(|_| RasterError::NoGhostscript)?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let status = wait_until(&mut child, Instant::now() + timeout);
    let (stdout, stderr) = (
        stdout.join().unwrap_or_default(),
        stderr.join().unwrap_or_default(),
    );
    match status {
        Some(status) => Ok(Output {
            status,
            stdout,
            stderr,
        }),
        None => Err(RasterError::Failed(format!(
            "Ghostscript did not finish within {} seconds",
            timeout.as_secs()
        ))),
    }
}

/// Read a child's stream on its own thread: the first [`MAX_TEXT_OUTPUT`]
/// bytes are kept and the rest discarded, so the child never blocks on a full
/// pipe.
fn drain(stream: Option<impl std::io::Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        if let Some(mut stream) = stream {
            let _ = (&mut stream).take(MAX_TEXT_OUTPUT).read_to_end(&mut kept);
            let _ = std::io::copy(&mut stream, &mut std::io::sink());
        }
        kept
    })
}

/// Wait for `child` until `deadline`, then kill it: its exit status, or
/// `None` when it was killed.
fn wait_until(child: &mut Child, deadline: Instant) -> Option<ExitStatus> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Bound the child's memory, CPU time, and file writes (Unix rlimits); a
/// limit the parent cannot set is left as it is.
#[cfg(unix)]
fn limit_resources(command: &mut Command, timeout: Duration) {
    use std::os::unix::process::CommandExt as _;
    let limits = [
        (libc::RLIMIT_AS, MAX_ADDRESS_SPACE),
        (libc::RLIMIT_CPU, timeout.as_secs().max(1)),
        (libc::RLIMIT_FSIZE, MAX_RASTER_FILE),
    ];
    // SAFETY: the hook runs between fork and exec, so it must not allocate or
    // lock; it only calls `setrlimit`, which is async-signal-safe, on a
    // stack array.
    unsafe {
        command.pre_exec(move || {
            for (resource, limit) in limits {
                let value = libc::rlimit {
                    rlim_cur: limit as libc::rlim_t,
                    rlim_max: limit as libc::rlim_t,
                };
                libc::setrlimit(resource, &value);
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn limit_resources(_command: &mut Command, _timeout: Duration) {}

/// `major.minor` from `gs --version` output such as `10.06.0`.
fn parse_version(text: &str) -> Option<(u32, u32)> {
    let mut parts = text.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// The input path and arguments Ghostscript is given. `--permit-file-read`
/// takes a list split on the platform's path-list separator, so an input
/// whose path holds one would be permitted as several partial paths; such an
/// input is copied into `scratch` first and the arguments point at the copy.
fn permitted_input(
    input: &Path,
    args: &[String],
    scratch: &ScratchDir,
) -> Result<(PathBuf, Vec<String>), RasterError> {
    let separator = if cfg!(windows) { ';' } else { ':' };
    let shown = input.display().to_string();
    if !shown.contains(separator) {
        return Ok((input.to_path_buf(), args.to_vec()));
    }
    let copy = scratch.path("input");
    std::fs::copy(input, &copy).map_err(|e| RasterError::Failed(e.to_string()))?;
    let renamed = copy.display().to_string();
    let args = args.iter().map(|a| a.replace(&shown, &renamed)).collect();
    Ok((copy, args))
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
            #[cfg(unix)]
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(not(unix))]
            let builder = std::fs::DirBuilder::new();
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

/// 64 bits from the operating system's random source.
fn random_u64() -> u64 {
    u64::from_le_bytes(crate::os_random())
}

/// Read a `pngalpha` page Ghostscript wrote, refusing a file over
/// [`MAX_RASTER_FILE`].
pub(crate) fn read_png(path: &Path) -> Result<Raster, RasterError> {
    let failed = |e: std::io::Error| RasterError::Failed(e.to_string());
    let file = std::fs::File::open(path).map_err(failed)?;
    let bytes = crate::budget::read_bounded(file, MAX_RASTER_FILE)
        .ok_or_else(|| RasterError::Failed("Ghostscript output too large".into()))?;
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
        format!("-dMaxBitmap={MAX_PAGE_BITMAP}"),
        format!("-sOutputFile={}", out.display()),
        "-f".into(),
        input.display().to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_arguments_band_large_pages() {
        let (out, input) = (Path::new("/o.png"), Path::new("/i.pdf"));
        for res in [72.0, 2400.0] {
            let args = png_args(res, res, true, out, input);
            let cap = args.iter().position(|a| a == "-dMaxBitmap=536870912");
            let file = args.iter().position(|a| a == "-f");
            assert!(cap.zip(file).is_some_and(|(c, f)| c < f), "{args:?}");
        }
    }

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

    /// An executable shell script in `scratch` standing in for `gs`.
    #[cfg(unix)]
    fn fake_gs(scratch: &ScratchDir, body: &str) -> Ghostscript {
        use std::os::unix::fs::PermissionsExt;
        let exe = scratch.path("gs");
        std::fs::write(&exe, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        Ghostscript { exe }
    }

    #[cfg(unix)]
    #[test]
    fn a_run_that_never_ends_is_killed() {
        // An infinite PostScript loop blocked a worker forever (#844 H7).
        let scratch = ScratchDir::new().unwrap();
        let gs = fake_gs(&scratch, "exec sleep 60");
        let started = Instant::now();
        let result = gs.run_within(
            Path::new("/x/in.pdf"),
            None,
            &[],
            Duration::from_millis(200),
        );
        assert!(
            matches!(&result, Err(RasterError::Failed(m)) if m.contains("did not finish")),
            "{:?}",
            result.err()
        );
        assert!(started.elapsed() < Duration::from_secs(20));
    }

    #[cfg(unix)]
    #[test]
    fn runaway_output_is_capped() {
        let scratch = ScratchDir::new().unwrap();
        let gs = fake_gs(&scratch, "head -c 40000000 /dev/zero");
        let run = gs.run_within(Path::new("/x/in.pdf"), None, &[], RUN_TIMEOUT);
        let run = run.unwrap_or_else(|e| panic!("{e}"));
        assert!(run.output.len() as u64 <= 2 * MAX_TEXT_OUTPUT + 1);
    }

    #[cfg(unix)]
    #[test]
    fn the_child_runs_under_resource_limits() {
        let scratch = ScratchDir::new().unwrap();
        // 4 GiB in 512-byte blocks and 8 GiB in KiB are both 8388608.
        let gs = fake_gs(&scratch, "ulimit -f; ulimit -v");
        let run = gs.run_within(Path::new("/x/in.pdf"), None, &[], RUN_TIMEOUT);
        let output = run.unwrap_or_else(|e| panic!("{e}")).output;
        assert_eq!(output.matches("8388608").count(), 2, "{output}");
    }

    #[cfg(unix)]
    #[test]
    fn an_old_ghostscript_is_refused() {
        let scratch = ScratchDir::new().unwrap();
        let old = fake_gs(&scratch, "echo 9.26");
        assert!(
            matches!(old.check_version(), Err(RasterError::Failed(m)) if m.contains("too old"))
        );
        let current = fake_gs(&scratch, "echo 10.06.0");
        assert_eq!(current.check_version(), Ok(()));
        assert_eq!(parse_version("9.55.0\n"), Some((9, 55)));
        assert_eq!(parse_version("junk"), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_path_holding_the_list_separator_is_read_through_a_copy() {
        let scratch = ScratchDir::new().unwrap();
        let input = scratch.path("a:b.pdf");
        std::fs::write(&input, b"%PDF-1.7").unwrap();
        let args = vec![format!("-sInputFile={}", input.display())];
        let (permitted, args) = permitted_input(&input, &args, &scratch).unwrap();
        assert!(!permitted.display().to_string().contains(':'));
        assert_eq!(std::fs::read(&permitted).unwrap(), b"%PDF-1.7");
        assert_eq!(args, vec![format!("-sInputFile={}", permitted.display())]);
        let plain = scratch.path("plain.pdf");
        let (kept, _) = permitted_input(&plain, &[], &scratch).unwrap();
        assert_eq!(kept, plain);
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
