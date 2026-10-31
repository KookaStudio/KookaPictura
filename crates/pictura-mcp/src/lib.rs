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

use std::io::{BufRead, Write};

/// Where the control server is, as the app computes it.
pub struct Endpoint {
    pub socket: String,
    pub token: String,
}

/// Serve MCP on `input` and `output` until `input` closes.
pub fn serve(input: impl BufRead, mut output: impl Write, backend: &mut dyn Backend) {
    for line in input.lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle_line(&line, backend) {
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
