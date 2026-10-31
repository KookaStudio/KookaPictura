//! The MCP stdio bridge behind `pictura --mcp`. It speaks MCP to a client and
//! the control protocol to a `ControlServer`, and enforces nothing itself:
//! permissions live in the server, which an agent can't talk around.

mod client;
mod server;
mod spawn;
mod tools;

pub use client::ControlClient;
pub use server::{handle_line, Backend, CallError};
pub use spawn::{parse_options, Options, Spawned};

use client::{read_line, Line};
use serde_json::json;
use std::io::{BufRead, Write};

/// The control socket refuses a longer request, so a line past this can't work.
const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// Where the control server is, as the app computes it.
pub struct Endpoint {
    pub socket: String,
    pub token_path: String,
}

/// Serve MCP on `input` and `output` until `input` closes.
pub fn serve(mut input: impl BufRead, mut output: impl Write, backend: &mut dyn Backend) {
    loop {
        let reply = match read_line(&mut input, MAX_REQUEST_BYTES) {
            Ok(Line::Complete(bytes)) => {
                let line = String::from_utf8_lossy(&bytes);
                if line.trim().is_empty() {
                    continue;
                }
                handle_line(&line, backend)
            }
            Ok(Line::TooLong) => Some(
                json!({"jsonrpc": "2.0", "id": null,
                       "error": {"code": -32600, "message": "the request is over 1 MiB"}})
                .to_string(),
            ),
            Ok(Line::Closed) | Err(_) => break,
        };
        if let Some(reply) = reply {
            if writeln!(output, "{reply}")
                .and_then(|()| output.flush())
                .is_err()
            {
                break;
            }
        }
    }
}

/// `pictura --mcp`: the process exit code.
pub fn run(endpoint: Endpoint, args: &[String]) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(problem) => {
            eprintln!("pictura mcp: {problem}");
            return 2;
        }
    };
    let (mut client, spawned) = if options.spawn {
        match Spawned::start(&options) {
            Ok(spawned) => {
                let endpoint = spawned.endpoint();
                (ControlClient::new(endpoint), Some(spawned))
            }
            Err(problem) => {
                eprintln!("pictura mcp: {problem}");
                return 1;
            }
        }
    } else {
        (ControlClient::new(endpoint), None)
    };
    // The child stops when this connection closes, so hold one from the start.
    if spawned.is_some() {
        if let Err(problem) = client.connect() {
            eprintln!("pictura mcp: {}", problem.message);
            return 1;
        }
    }
    let stdin = std::io::stdin();
    serve(stdin.lock(), std::io::stdout(), &mut client);
    drop(client);
    drop(spawned);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::io::Cursor;

    struct Silent;

    impl Backend for Silent {
        fn call(&mut self, _method: &str, _params: Value) -> Result<Value, CallError> {
            Ok(json!({}))
        }
    }

    #[test]
    fn an_oversized_request_is_refused_and_the_bridge_continues() {
        let mut input = vec![b'x'; MAX_REQUEST_BYTES + 1];
        input.push(b'\n');
        input.extend_from_slice(b"{\"id\":2,\"method\":\"ping\"}\n");
        let mut output = Vec::new();
        serve(Cursor::new(input), &mut output, &mut Silent);
        let replies: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0]["error"]["code"], -32600);
        assert_eq!(replies[1]["id"], 2);
        assert!(replies[1]["result"].is_object());
    }
}
