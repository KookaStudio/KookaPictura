use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command as StdCommand, Stdio};
use std::sync::{Mutex, OnceLock};

const MAX_REPLY: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Magick,
    MagickRsvg,
    PsdTools,
    PsdToolsSkimage,
    PsdToolsShallow,
    AgPsd,
}

impl Tool {
    fn family(self) -> &'static str {
        match self {
            Self::Magick | Self::MagickRsvg => "magick",
            Self::PsdTools | Self::PsdToolsSkimage | Self::PsdToolsShallow => "psd-tools",
            Self::AgPsd => "ag-psd",
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Self::Magick => "magick not available",
            Self::MagickRsvg => "magick RSVG coder not available",
            Self::PsdTools => "python3 + psd-tools not available",
            Self::PsdToolsSkimage => "python3 + psd-tools + skimage not available",
            Self::PsdToolsShallow => "python3 + psd-tools>=1.19 not available",
            Self::AgPsd => "node + ag-psd not available",
        }
    }

    fn cache(self) -> &'static OnceLock<bool> {
        match self {
            Self::Magick => {
                static AVAILABLE: OnceLock<bool> = OnceLock::new();
                &AVAILABLE
            }
            Self::MagickRsvg => {
                static AVAILABLE: OnceLock<bool> = OnceLock::new();
                &AVAILABLE
            }
            Self::PsdTools => {
                static AVAILABLE: OnceLock<bool> = OnceLock::new();
                &AVAILABLE
            }
            Self::PsdToolsSkimage => {
                static AVAILABLE: OnceLock<bool> = OnceLock::new();
                &AVAILABLE
            }
            Self::PsdToolsShallow => {
                static AVAILABLE: OnceLock<bool> = OnceLock::new();
                &AVAILABLE
            }
            Self::AgPsd => {
                static AVAILABLE: OnceLock<bool> = OnceLock::new();
                &AVAILABLE
            }
        }
    }

    fn probe(self) -> bool {
        match self {
            Self::Magick => StdCommand::new("magick")
                .arg("-version")
                .output()
                .is_ok_and(|out| out.status.success()),
            Self::MagickRsvg => StdCommand::new("magick")
                .args(["-list", "format"])
                .output()
                .is_ok_and(|out| {
                    String::from_utf8_lossy(&out.stdout).lines().any(|line| {
                        line.trim_start().starts_with("RSVG") && line.contains("Librsvg")
                    })
                }),
            Self::PsdTools => StdCommand::new("python3")
                .args(["-c", "import psd_tools"])
                .output()
                .is_ok_and(|out| out.status.success()),
            Self::PsdToolsSkimage => StdCommand::new("python3")
                .args(["-c", "import psd_tools, skimage, numpy"])
                .output()
                .is_ok_and(|out| out.status.success()),
            Self::PsdToolsShallow => StdCommand::new("python3")
                .args([
                    "-c",
                    "import importlib.metadata as m, sys; \
                     v = tuple(int(p) for p in m.version('psd-tools').split('.')[:2]); \
                     sys.exit(0 if v >= (1, 19) else 1)",
                ])
                .output()
                .is_ok_and(|out| out.status.success()),
            Self::AgPsd => StdCommand::new("node")
                .args(["-e", "require('ag-psd')"])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .is_ok_and(|out| out.status.success()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleSkip {
    tool: Tool,
}

impl OracleSkip {
    pub fn family(&self) -> &'static str {
        self.tool.family()
    }

    pub fn reason(&self) -> &'static str {
        self.tool.reason()
    }

    pub fn token(&self, test: &str) -> String {
        format!(
            "pictura oracle: SKIP {} {} {}",
            self.family(),
            test,
            self.reason()
        )
    }

    pub fn report(self) {
        let test = format!("{}::{}", binary_id(), test_name());
        let line = format!("{}\n", self.token(&test));
        eprint!("{line}");
        if let Some(path) = std::env::var_os("PICTURA_ORACLE_SKIPS") {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("append oracle skip token");
            file.write_all(line.as_bytes())
                .expect("append oracle skip token");
        }
    }
}

fn require_with(tool: Tool, available: bool) -> Result<(), OracleSkip> {
    if available {
        Ok(())
    } else {
        Err(OracleSkip { tool })
    }
}

pub fn require(tool: Tool) -> Result<(), OracleSkip> {
    require_with(tool, *tool.cache().get_or_init(|| tool.probe()))
}

fn test_name() -> String {
    std::thread::current()
        .name()
        .unwrap_or("unknown")
        .to_string()
}

fn binary_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        if let Ok(id) = std::env::var("NEXTEST_BINARY_ID") {
            return id;
        }
        let package = env!("CARGO_PKG_NAME");
        let normalized = package.replace('-', "_");
        let stem = std::env::current_exe()
            .ok()
            .and_then(|path| path.file_stem().map(OsStr::to_owned))
            .and_then(|stem| stem.into_string().ok())
            .unwrap_or_else(|| normalized.clone());
        let target = stem
            .rsplit_once('-')
            .map_or(stem.as_str(), |(base, _)| base);
        if target == normalized {
            package.to_string()
        } else {
            format!("{package}::{target}")
        }
    })
}

#[macro_export]
macro_rules! require_oracle {
    ($tool:expr) => {
        if let Err(skip) = $crate::oracle::require($tool) {
            skip.report();
            return;
        }
    };
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Program {
    #[default]
    Python,
    Node,
}

impl Program {
    fn new(value: &OsStr) -> Self {
        match value.to_str() {
            Some("python3") => Self::Python,
            Some("node") => Self::Node,
            _ => panic!("oracle transport supports only python3 and node"),
        }
    }

    fn executable(self) -> &'static str {
        match self {
            Self::Python => "python3",
            Self::Node => "node",
        }
    }

    fn worker(self) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts")
            .join(match self {
                Self::Python => "oracle_batch.py",
                Self::Node => "oracle_batch.js",
            })
    }

    fn slot(self) -> &'static Mutex<Option<Worker>> {
        match self {
            Self::Python => {
                static SLOT: Mutex<Option<Worker>> = Mutex::new(None);
                &SLOT
            }
            Self::Node => {
                static SLOT: Mutex<Option<Worker>> = Mutex::new(None);
                &SLOT
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status(i32);

impl Status {
    pub fn success(&self) -> bool {
        self.0 == 0
    }

    pub fn code(&self) -> Option<i32> {
        Some(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Command {
    program: Program,
    args: Vec<OsString>,
    env: Vec<(OsString, Option<OsString>)>,
    dir: Option<OsString>,
}

impl Command {
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: Program::new(program.as_ref()),
            ..Self::default()
        }
    }

    pub fn arg(&mut self, value: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(value.as_ref().to_owned());
        self
    }

    pub fn args<I, S>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for value in values {
            self.arg(value);
        }
        self
    }

    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env
            .push((key.as_ref().to_owned(), Some(value.as_ref().to_owned())));
        self
    }

    pub fn env_remove<K>(&mut self, key: K) -> &mut Self
    where
        K: AsRef<OsStr>,
    {
        self.env.push((key.as_ref().to_owned(), None));
        self
    }

    pub fn current_dir<P>(&mut self, dir: P) -> &mut Self
    where
        P: AsRef<OsStr>,
    {
        self.dir = Some(dir.as_ref().to_owned());
        self
    }

    pub fn output(&self) -> io::Result<Output> {
        let (mut slot, poisoned) = match self.program.slot().lock() {
            Ok(slot) => (slot, false),
            Err(error) => (error.into_inner(), true),
        };
        if poisoned {
            *slot = None;
        }
        let result = match slot.as_mut() {
            Some(worker) => worker.request(&self.args, &self.env, self.dir.as_deref()),
            None => {
                let mut worker = Worker::spawn(self.program)?;
                let result = worker.request(&self.args, &self.env, self.dir.as_deref());
                if result.is_ok() {
                    *slot = Some(worker);
                }
                result
            }
        };
        if result.is_err() {
            *slot = None;
        }
        result
    }

    pub fn status(&self) -> io::Result<Output> {
        self.output()
    }
}

struct Worker {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl Worker {
    fn spawn(program: Program) -> io::Result<Self> {
        let mut child = StdCommand::new(program.executable())
            .arg(program.worker())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("oracle worker has no stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("oracle worker has no stdout"))?;
        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    fn request(
        &mut self,
        args: &[OsString],
        env: &[(OsString, Option<OsString>)],
        dir: Option<&OsStr>,
    ) -> io::Result<Output> {
        write_request(&mut self.stdin, args, env, dir)?;
        let mut header = [0u8; 20];
        self.stdout.read_exact(&mut header)?;
        let code = i32::from_le_bytes(header[..4].try_into().unwrap());
        let stdout_len = u64::from_le_bytes(header[4..12].try_into().unwrap());
        let stderr_len = u64::from_le_bytes(header[12..20].try_into().unwrap());
        if stdout_len > MAX_REPLY || stderr_len > MAX_REPLY {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "oracle worker reply exceeds the size limit",
            ));
        }
        let mut stdout = vec![0u8; stdout_len as usize];
        let mut stderr = vec![0u8; stderr_len as usize];
        self.stdout.read_exact(&mut stdout)?;
        self.stdout.read_exact(&mut stderr)?;
        Ok(Output {
            status: Status(code),
            stdout,
            stderr,
        })
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn os_bytes(value: &OsStr) -> io::Result<Vec<u8>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(value.as_bytes().to_vec())
    }
    #[cfg(not(unix))]
    {
        value
            .to_str()
            .map(|text| text.as_bytes().to_vec())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "non-Unicode oracle argument")
            })
    }
}

fn write_u64(writer: &mut impl Write, value: u64) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_field(writer: &mut impl Write, value: &[u8]) -> io::Result<()> {
    write_u64(writer, value.len() as u64)?;
    writer.write_all(value)
}

fn write_request(
    writer: &mut impl Write,
    args: &[OsString],
    env: &[(OsString, Option<OsString>)],
    dir: Option<&OsStr>,
) -> io::Result<()> {
    match dir {
        Some(dir) => write_field(writer, &os_bytes(dir)?)?,
        None => write_u64(writer, u64::MAX)?,
    }
    write_u64(writer, args.len() as u64)?;
    for arg in args {
        write_field(writer, &os_bytes(arg)?)?;
    }
    write_u64(writer, env.len() as u64)?;
    for (key, value) in env {
        write_field(writer, &os_bytes(key)?)?;
        match value {
            Some(value) => {
                writer.write_all(&[1])?;
                write_field(writer, &os_bytes(value)?)?;
            }
            None => writer.write_all(&[0])?,
        }
    }
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(output: &Output) -> String {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn missing_tool_returns_typed_skip() {
        let skip = require_with(Tool::Magick, false).unwrap_err();
        assert_eq!(skip.family(), "magick");
        assert_eq!(binary_id(), env!("CARGO_PKG_NAME"));
        assert_eq!(
            skip.token("pictura-testkit::oracle::missing_tool_returns_typed_skip"),
            "pictura oracle: SKIP magick pictura-testkit::oracle::missing_tool_returns_typed_skip magick not available"
        );
    }

    #[test]
    fn capability_variants_keep_tool_family() {
        for (tool, family, reason) in [
            (Tool::MagickRsvg, "magick", "RSVG"),
            (Tool::PsdToolsSkimage, "psd-tools", "skimage"),
            (Tool::PsdToolsShallow, "psd-tools", "1.19"),
        ] {
            let skip = require_with(tool, false).unwrap_err();
            assert_eq!(skip.family(), family);
            assert!(skip.reason().contains(reason));
            assert!(skip.token("binary::test").ends_with(skip.reason()));
        }
    }

    #[test]
    fn python_batch_replies_stay_in_request_order() {
        let script = "import os,sys; print(os.getpid(), '|'.join(sys.argv[1:]))";
        let first = Command::new("python3")
            .args(["-c", script, "first", "a"])
            .output()
            .unwrap();
        let second = Command::new("python3")
            .args(["-c", script, "second", "b"])
            .output()
            .unwrap();
        assert!(first.status.success(), "{first:?}");
        assert!(second.status.success(), "{second:?}");
        let first = text(&first);
        let second = text(&second);
        let (first_pid, first_args) = first.split_once(' ').unwrap();
        let (second_pid, second_args) = second.split_once(' ').unwrap();
        assert_eq!(first_pid, second_pid);
        assert_eq!(first_args, "first|a");
        assert_eq!(second_args, "second|b");
    }

    #[test]
    fn node_batch_replies_stay_in_request_order() {
        require_oracle!(Tool::AgPsd);
        let script = "console.log(process.pid, process.argv.slice(1).join('|'))";
        let first = Command::new("node")
            .args(["-e", script, "first", "a"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .unwrap();
        let second = Command::new("node")
            .args(["-e", script, "second", "b"])
            .output()
            .unwrap();
        assert!(first.status.success(), "{first:?}");
        assert!(second.status.success(), "{second:?}");
        let first = text(&first);
        let second = text(&second);
        let (first_pid, first_args) = first.split_once(' ').unwrap();
        let (second_pid, second_args) = second.split_once(' ').unwrap();
        assert_eq!(first_pid, second_pid);
        assert_eq!(first_args, "first|a");
        assert_eq!(second_args, "second|b");
    }

    #[test]
    fn python_worker_preserves_status_and_survives() {
        let failed = Command::new("python3")
            .args([
                "-c",
                "import os,sys; print(os.getpid()); print('out'); print('err', file=sys.stderr); sys.exit(3)",
            ])
            .output()
            .unwrap();
        assert_eq!(failed.status.code(), Some(3));
        assert_eq!(String::from_utf8_lossy(&failed.stderr).trim(), "err");
        let stdout = String::from_utf8_lossy(&failed.stdout);
        let mut lines = stdout.lines();
        let pid = lines.next().unwrap();
        assert_eq!(lines.next(), Some("out"));
        assert!(lines.next().is_none());

        let recovered = Command::new("python3")
            .args(["-c", "import os; print(os.getpid())"])
            .output()
            .unwrap();
        assert!(recovered.status.success(), "{recovered:?}");
        assert_eq!(text(&recovered), pid);
    }

    #[test]
    fn node_worker_preserves_status_and_survives() {
        require_oracle!(Tool::AgPsd);
        let failed = Command::new("node")
            .args([
                "-e",
                "console.log(process.pid); console.log('out'); console.error('err'); process.exit(3)",
            ])
            .output()
            .unwrap();
        assert_eq!(failed.status.code(), Some(3));
        assert_eq!(String::from_utf8_lossy(&failed.stderr).trim(), "err");
        let stdout = String::from_utf8_lossy(&failed.stdout);
        let mut lines = stdout.lines();
        let pid = lines.next().unwrap();
        assert_eq!(lines.next(), Some("out"));
        assert!(lines.next().is_none());

        let recovered = Command::new("node")
            .args(["-e", "console.log(process.pid)"])
            .output()
            .unwrap();
        assert!(recovered.status.success(), "{recovered:?}");
        assert_eq!(text(&recovered), pid);
    }
}
