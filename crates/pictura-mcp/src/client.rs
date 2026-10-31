//! The control socket. Reads are bounded and tolerate late replies, so one bad
//! reply never desynchronises the stream.

use crate::server::{Backend, CallError};
use crate::Endpoint;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::time::Duration;

const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;
// Above the server's 60 s approval timeout, and slow filters run on large images.
const CALL_TIMEOUT: Duration = Duration::from_secs(300);

const NOT_RUNNING: &str = "Kooka Pictura is not running, or Agent Control is off \
(Edit > Preferences > Agent Control)";

struct Connection {
    reader: BufReader<Box<dyn Read + Send>>,
    writer: Box<dyn Write + Send>,
    token: String,
}

pub struct ControlClient {
    endpoint: Endpoint,
    connection: Option<Connection>,
    next_id: i64,
}

pub(crate) enum Line {
    Complete(Vec<u8>),
    TooLong,
    Closed,
}

/// One line, consuming a line over `max` to its newline so the next one is clean.
pub(crate) fn read_line(reader: &mut impl BufRead, max: usize) -> std::io::Result<Line> {
    let mut line = Vec::new();
    let mut too_long = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(if line.is_empty() {
                Line::Closed
            } else {
                Line::Complete(line)
            });
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(available.len());
        if line.len() + take > max {
            too_long = true;
            line.clear();
        } else if !too_long {
            line.extend_from_slice(&available[..take]);
        }
        let consumed = newline.map_or(take, |at| at + 1);
        reader.consume(consumed);
        if newline.is_some() {
            return Ok(if too_long {
                Line::TooLong
            } else {
                Line::Complete(line)
            });
        }
    }
}

#[cfg(unix)]
fn check_ownership(endpoint: &Endpoint) -> Result<(), CallError> {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: geteuid has no preconditions and cannot fail.
    let user = unsafe { libc::geteuid() };
    let foreign =
        |path: &str| CallError::new("not_running", format!("{path} belongs to another user"));
    let socket = std::fs::metadata(&endpoint.socket)
        .map_err(|_| CallError::new("not_running", NOT_RUNNING))?;
    if socket.uid() != user {
        return Err(foreign(&endpoint.socket));
    }
    let token = std::fs::metadata(&endpoint.token_path)
        .map_err(|_| CallError::new("not_running", NOT_RUNNING))?;
    if token.uid() != user {
        return Err(foreign(&endpoint.token_path));
    }
    if token.mode() & 0o077 != 0 {
        return Err(CallError::new(
            "not_running",
            format!("{} is readable by others", endpoint.token_path),
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_ownership(_endpoint: &Endpoint) -> Result<(), CallError> {
    Ok(())
}

#[cfg(unix)]
fn open_socket(
    endpoint: &Endpoint,
) -> std::io::Result<(Box<dyn Read + Send>, Box<dyn Write + Send>)> {
    let stream = std::os::unix::net::UnixStream::connect(&endpoint.socket)?;
    stream.set_read_timeout(Some(CALL_TIMEOUT))?;
    Ok((Box::new(stream.try_clone()?), Box::new(stream)))
}

#[cfg(not(unix))]
fn open_socket(
    endpoint: &Endpoint,
) -> std::io::Result<(Box<dyn Read + Send>, Box<dyn Write + Send>)> {
    let pipe = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!(r"\\.\pipe\{}", endpoint.socket))?;
    Ok((Box::new(pipe.try_clone()?), Box::new(pipe)))
}

impl ControlClient {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            connection: None,
            next_id: 0,
        }
    }

    pub fn connect(&mut self) -> Result<(), CallError> {
        if self.connection.is_some() {
            return Ok(());
        }
        if self.endpoint.socket.is_empty() {
            return Err(CallError::new("not_running", NOT_RUNNING));
        }
        check_ownership(&self.endpoint)?;
        let not_running = |_| CallError::new("not_running", NOT_RUNNING);
        let token = std::fs::read_to_string(&self.endpoint.token_path).map_err(not_running)?;
        let (reader, writer) = open_socket(&self.endpoint).map_err(not_running)?;
        self.connection = Some(Connection {
            reader: BufReader::new(reader),
            writer,
            token: token.trim().to_string(),
        });
        Ok(())
    }

    fn exchange(&mut self, method: &str, params: Value) -> Result<Value, CallError> {
        self.next_id += 1;
        let id = self.next_id;
        let Some(connection) = self.connection.as_mut() else {
            return Err(CallError::new("not_running", NOT_RUNNING));
        };
        let request =
            json!({"id": id, "method": method, "params": params, "token": connection.token});
        let sent =
            writeln!(connection.writer, "{request}").and_then(|()| connection.writer.flush());
        if sent.is_err() {
            self.connection = None;
            return Err(CallError::new(
                "disconnected",
                "the control connection closed",
            ));
        }
        loop {
            let line = match read_line(&mut connection.reader, MAX_REPLY_BYTES) {
                Ok(line) => line,
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                {
                    // The reply may still come; its id gets it skipped then.
                    return Err(CallError::new(
                        "timeout",
                        "no reply in time; the operation may still complete, so check \
                         the state before retrying",
                    ));
                }
                Err(_) => {
                    self.connection = None;
                    return Err(CallError::new(
                        "disconnected",
                        "the control connection closed",
                    ));
                }
            };
            let bytes = match line {
                Line::Complete(bytes) => bytes,
                Line::TooLong => {
                    return Err(CallError::new(
                        "too_large",
                        "the reply is over 16 MiB; ask for less",
                    ));
                }
                Line::Closed => {
                    self.connection = None;
                    return Err(CallError::new(
                        "disconnected",
                        "the control connection closed",
                    ));
                }
            };
            let Ok(reply) = serde_json::from_slice::<Value>(&bytes) else {
                continue;
            };
            if reply.get("id").and_then(Value::as_i64) != Some(id) {
                continue;
            }
            if reply.get("ok").and_then(Value::as_bool) == Some(true) {
                return Ok(reply.get("result").cloned().unwrap_or(json!({})));
            }
            let error = reply.get("error").cloned().unwrap_or(Value::Null);
            let text = |key: &str| {
                error
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string()
            };
            return Err(CallError {
                code: text("code"),
                message: text("message"),
            });
        }
    }
}

impl Backend for ControlClient {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, CallError> {
        self.connect()?;
        self.exchange(method, params)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Cursor, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;

    struct Fixture {
        directory: PathBuf,
        endpoint: Endpoint,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }

    fn fixture(name: &str, token_mode: u32) -> (Fixture, UnixListener) {
        let directory =
            std::env::temp_dir().join(format!("pictura-mcp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir(&directory).unwrap();
        let socket = directory.join("c.sock");
        let token = directory.join("c.sock.token");
        std::fs::write(&token, "secret").unwrap();
        std::fs::set_permissions(&token, std::fs::Permissions::from_mode(token_mode)).unwrap();
        let listener = UnixListener::bind(&socket).unwrap();
        let endpoint = Endpoint {
            socket: socket.to_str().unwrap().to_string(),
            token_path: token.to_str().unwrap().to_string(),
        };
        (
            Fixture {
                directory,
                endpoint,
            },
            listener,
        )
    }

    fn client_of(fixture: &Fixture) -> ControlClient {
        ControlClient::new(Endpoint {
            socket: fixture.endpoint.socket.clone(),
            token_path: fixture.endpoint.token_path.clone(),
        })
    }

    #[test]
    fn nothing_listening_names_preferences() {
        let mut client = ControlClient::new(Endpoint {
            socket: "/nonexistent/live.sock".to_string(),
            token_path: "/nonexistent/live.sock.token".to_string(),
        });
        let error = client.call("status", json!({})).unwrap_err();
        assert!(error.message.contains("Preferences"));
    }

    #[test]
    fn an_exposed_token_is_refused() {
        let (fixture, _listener) = fixture("exposed", 0o644);
        let error = client_of(&fixture).call("status", json!({})).unwrap_err();
        assert!(error.message.contains("readable by others"));
    }

    #[test]
    fn a_huge_reply_does_not_break_the_stream() {
        let huge = vec![b'x'; 40];
        let mut input = huge.clone();
        input.push(b'\n');
        input.extend_from_slice(b"short\n");
        let mut reader = Cursor::new(input);
        assert!(matches!(read_line(&mut reader, 10).unwrap(), Line::TooLong));
        match read_line(&mut reader, 10).unwrap() {
            Line::Complete(line) => assert_eq!(line, b"short"),
            _ => panic!("expected the next line"),
        }
        assert!(matches!(read_line(&mut reader, 10).unwrap(), Line::Closed));
    }

    #[test]
    fn a_late_reply_is_skipped() {
        let (fixture, listener) = fixture("late", 0o600);
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let request: Value = serde_json::from_str(&request).unwrap();
            assert_eq!(request["token"], "secret");
            let id = request["id"].as_i64().unwrap();
            let mut stream = stream;
            writeln!(
                stream,
                "{}",
                json!({"id": id - 1, "ok": true, "result": {"old": true}})
            )
            .unwrap();
            writeln!(stream, "not json").unwrap();
            writeln!(
                stream,
                "{}",
                json!({"id": id, "ok": true, "result": {"new": true}})
            )
            .unwrap();
        });
        let result = client_of(&fixture).call("status", json!({})).unwrap();
        assert_eq!(result, json!({"new": true}));
        server.join().unwrap();
    }

    #[test]
    fn a_control_refusal_keeps_its_code() {
        let (fixture, listener) = fixture("refused", 0o600);
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let id = serde_json::from_str::<Value>(&request).unwrap()["id"]
                .as_i64()
                .unwrap();
            let mut stream = stream;
            let reply =
                json!({"id": id, "ok": false, "error": {"code": "refused", "message": "no"}});
            writeln!(stream, "{reply}").unwrap();
        });
        let error = client_of(&fixture).call("filter", json!({})).unwrap_err();
        assert_eq!(error, CallError::new("refused", "no"));
        server.join().unwrap();
    }
}
