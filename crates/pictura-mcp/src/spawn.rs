//! `--spawn`: a private headless instance, instead of attaching to a window.

use crate::Endpoint;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const START_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub spawn: bool,
    pub tier: Option<String>,
    pub roots: Vec<String>,
    pub allow_all: bool,
}

pub fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--spawn" => options.spawn = true,
            "--tier" => {
                let tier = value()?;
                if !["read", "edit", "save"].contains(&tier.as_str()) {
                    return Err(format!("unknown tier: {tier}"));
                }
                options.tier = Some(tier);
            }
            "--root" => options.roots.push(value()?),
            "--approval" => {
                let approval = value()?;
                if approval != "allow-all" {
                    return Err(format!("unknown approval: {approval}"));
                }
                options.allow_all = true;
            }
            _ => return Err(format!("unknown option: {arg}")),
        }
    }
    let spawn_only = options.tier.is_some() || !options.roots.is_empty() || options.allow_all;
    if spawn_only && !options.spawn {
        return Err("--tier, --root and --approval need --spawn".to_string());
    }
    Ok(options)
}

fn child_arguments(options: &Options, socket: &str, state: &str) -> Vec<String> {
    let mut args: Vec<String> = [
        "--headless",
        "--control",
        "--control-exit-on-disconnect",
        "--control-socket",
        socket,
        "--state-home",
        state,
        "--control-tier",
        options.tier.as_deref().unwrap_or("read"),
    ]
    .map(str::to_string)
    .to_vec();
    for root in &options.roots {
        args.push("--control-root".to_string());
        args.push(root.clone());
    }
    if options.allow_all {
        args.push("--control-approval".to_string());
        args.push("allow-all".to_string());
    }
    args
}

fn program() -> Result<PathBuf, String> {
    // An AppImage's own path is a temporary mount.
    if let Some(image) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(image));
    }
    std::env::current_exe().map_err(|error| error.to_string())
}

fn private_directory() -> Result<PathBuf, String> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.subsec_nanos());
    let path = std::env::temp_dir().join(format!("pictura-mcp-{}-{nanos}", std::process::id()));
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder
        .create(&path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    Ok(path)
}

/// The child, and its directory. Both are gone when this drops.
pub struct Spawned {
    child: Child,
    directory: PathBuf,
    socket: String,
    token: String,
}

impl Spawned {
    pub fn start(options: &Options) -> Result<Self, String> {
        let directory = private_directory()?;
        let state = directory.join("state");
        #[cfg(unix)]
        let (socket, token) = {
            let socket = directory.join("c.sock").to_string_lossy().into_owned();
            let token = format!("{socket}.token");
            (socket, token)
        };
        // A pipe name, and the server puts its token in the temp directory.
        #[cfg(not(unix))]
        let (socket, token) = {
            let socket = format!("pictura-mcp-{}", std::process::id());
            let token = std::env::temp_dir()
                .join(format!("{socket}.token"))
                .to_string_lossy()
                .into_owned();
            (socket, token)
        };
        let child = Command::new(program()?)
            .args(child_arguments(options, &socket, &state.to_string_lossy()))
            .stdin(Stdio::null())
            // Stdout is the protocol's, and the child's output is not part of it.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("cannot start: {error}"))?;
        let mut spawned = Self {
            child,
            directory,
            socket,
            token,
        };
        spawned.wait_until_ready()?;
        Ok(spawned)
    }

    fn wait_until_ready(&mut self) -> Result<(), String> {
        let started = Instant::now();
        while started.elapsed() < START_TIMEOUT {
            if std::fs::metadata(&self.token).is_ok_and(|token| token.len() > 0) {
                return Ok(());
            }
            if let Ok(Some(status)) = self.child.try_wait() {
                return Err(format!("the instance exited: {status}"));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err("the instance did not start in time".to_string())
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint {
            socket: self.socket.clone(),
            token: self.token.clone(),
        }
    }
}

impl Drop for Spawned {
    fn drop(&mut self) {
        // The closed connection stops the child; this catches one that hung.
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if !matches!(self.child.try_wait(), Ok(None)) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_options(&args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn spawn_arguments_come_from_the_options() {
        let options =
            parse(&["--spawn", "--tier", "edit", "--root", "/a", "--root", "/b"]).unwrap();
        let args = child_arguments(&options, "/s", "/state").join(" ");
        assert!(args.contains("--headless --control --control-exit-on-disconnect"));
        assert!(args.contains("--control-socket /s --state-home /state"));
        assert!(args.contains("--control-tier edit"));
        assert!(args.contains("--control-root /a --control-root /b"));
        assert!(!args.contains("--control-approval"));
    }

    #[test]
    fn the_default_tier_is_read() {
        let options = parse(&["--spawn"]).unwrap();
        assert!(child_arguments(&options, "/s", "/state")
            .join(" ")
            .contains("--control-tier read"));
    }

    #[test]
    fn bad_arguments_are_refused() {
        assert!(parse(&["--nope"]).is_err());
        assert!(parse(&["--spawn", "--tier", "admin"]).is_err());
        assert!(parse(&["--spawn", "--approval", "maybe"]).is_err());
        assert!(parse(&["--tier", "edit"]).is_err());
        assert!(parse(&["--spawn", "--root"]).is_err());
        assert_eq!(parse(&[]).unwrap(), Options::default());
    }
}
