//! # XSCP Client CLI
//!
//! This binary provides a CLI to send simple messages to an XSCP server over TCP.

use ::io::SocketIo;
use client::{auth, clear_prompt_line, print_prompt};
use std::io::Write;
use std::ops::ControlFlow;
use tokio::io::{self, AsyncBufReadExt, BufReader, Lines, Stdin};
use tokio::net::TcpStream;
use xscp::{OpCode, XscpNotification, XscpRequest, XscpResponse};

#[tokio::main]
async fn main() {
    let socket = TcpStream::connect("127.0.0.1:7878")
        .await
        .expect("Failed to connect to server");
    let mut socket_io = SocketIo::new(socket);
    println!("Connected to 127.0.0.1:7878");

    let mut stdin_lines = BufReader::new(io::stdin()).lines();

    // Auth
    let username = run_auth(&mut stdin_lines, &mut socket_io).await;
    println!("Logged in as {username}!");
    print_prompt();

    // Active Connection
    loop {
        let flow = tokio::select! {
            stdin_line = stdin_lines.next_line() => { // User typed a line on stdin
                handle_stdin_line(stdin_line, &mut socket_io, &username).await
            }
            read_result = socket_io.read() => { // Server sent a PDU over the socket
                handle_server_message(read_result)
            }
        };

        if flow.is_break() {
            return;
        }
    }
}

/// Processes a line typed by the user on stdin.
///
/// A non-empty line is sent to the server as an [`OpCode::Send`] request.
/// Reaching EOF (Ctrl-D) sends a best-effort `EXIT` and signals
/// [`ControlFlow::Break`] so the client shuts down cleanly.
async fn handle_stdin_line(
    stdin_line: io::Result<Option<String>>,
    socket_io: &mut SocketIo,
    username: &str,
) -> ControlFlow<()> {
    let line = match stdin_line {
        Ok(Some(line)) => line,
        Ok(None) => {
            // EOF (Ctrl-D): tell the server we're leaving, then stop.
            if let Ok(exit) = XscpRequest::try_new(OpCode::Exit, username, "") {
                let _ = socket_io.write(&exit.to_string()).await;
            }
            println!("\nExiting...");
            return ControlFlow::Break(());
        }
        Err(err) => {
            eprintln!("Error reading stdin: {err}");
            return ControlFlow::Break(());
        }
    };

    let request = match XscpRequest::try_new(OpCode::Send, username, &line) {
        Ok(req) => req,
        Err(err) => {
            eprintln!("Cannot send message: {err:?}");
            print_prompt();
            return ControlFlow::Continue(());
        }
    };

    if let Err(err) = socket_io.write(&request.to_string()).await {
        eprintln!("Failed to send message: {err}");
        return ControlFlow::Break(());
    }

    print_prompt();
    ControlFlow::Continue(())
}

/// Processes a PDU read from the server socket.
///
/// The server may send either an [`XscpNotification`] (a `BRDC` broadcast from
/// another client) or an [`XscpResponse`] (e.g. the `200 Ok` acknowledging our
/// own `SEND`). Returns [`ControlFlow::Break`] when the server closes the
/// connection (EOF) or the read fails.
fn handle_server_message(
    read_result: Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>,
) -> ControlFlow<()> {
    let raw = match read_result {
        Ok(Some(raw)) => raw,
        Ok(None) => {
            clear_prompt_line();
            println!("Server closed the connection");
            return ControlFlow::Break(());
        }
        Err(err) => {
            clear_prompt_line();
            eprintln!("Failed to read from server: {err}");
            return ControlFlow::Break(());
        }
    };

    match XscpNotification::parse(&raw) {
        Ok(notification) => {
            clear_prompt_line();
            println!("{}: {}", notification.source(), notification.message());
            print_prompt();
        }
        // Not a notification: it should be a response to one of our requests.
        Err(_) => match XscpResponse::parse(&raw) {
            Ok(response) if response.status_code() != 200 => {
                clear_prompt_line();
                eprintln!(
                    "Error {}: {}",
                    response.status_code(),
                    response.reason_phrase()
                );
                print_prompt();
            }
            Ok(_) => {} // 200 Ok: our message was accepted, nothing to show.
            Err(err) => {
                clear_prompt_line();
                eprintln!("Received unparseable PDU from server: {err:?}");
                print_prompt();
            }
        },
    }

    ControlFlow::Continue(())
}

/// Prompts for a username and authenticates against the server until it
/// succeeds, returning the authenticated username.
///
/// Borrows `stdin_lines` and `socket_io` so the caller can keep using them
/// afterwards. Terminates the process on a fatal outcome (auth error, EOF, or
/// an unexpected status code).
async fn run_auth(stdin_lines: &mut Lines<BufReader<Stdin>>, socket_io: &mut SocketIo) -> String {
    loop {
        print!("Username:");
        std::io::stdout().flush().expect("Failed to flush stdout");

        let line = stdin_lines.next_line().await;
        match line.expect("Error reading stdin") {
            Some(text) => match auth(socket_io, &text).await {
                Ok(200) => return text,
                Ok(401) => {
                    println!("Invalid Credentials, try again");
                    continue;
                }
                Ok(402) => {
                    println!("Exceeded auth attempts");
                    std::process::exit(1);
                }
                Ok(code) => {
                    eprintln!("Unexpected server response (status {code}), exiting...");
                    std::process::exit(1);
                }
                Err(err) => {
                    eprintln!("Authentication failed: {err}");
                    std::process::exit(1);
                }
            },
            None => {
                println!("\nExiting...");
                std::process::exit(0);
            }
        }
    }
}
