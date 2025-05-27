//! # R-Client - Remote Command Line Client
//!
//! `r_client` is a command-line application that connects to the `r_server`
//! to send commands and receive responses. It supports interactive input
//! for commands like `ECHO <message>` and `LS`.
//!
//! ## Features
//! - Connects to a hardcoded server address (`127.0.0.1:8080`).
//! - Prompts the user for commands in a loop.
//! - Sends user commands to the server.
//! - Receives and displays server responses.
//! - Handles multi-line responses from the server for the `LS` command
//!   (terminated by a blank line).
//! - Allows disconnection using "exit" or "quit" commands.
//! - Uses Tokio for asynchronous network I/O and `env_logger` for logging.

use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::io::{stdin, stdout, Write}; // For handling standard input/output
use log::{info, debug, error};

/// The main entry point for the R-Client application.
///
/// This function orchestrates the client's lifecycle:
/// 1.  Initializes `env_logger`.
/// 2.  Defines the server address (currently hardcoded to `127.0.0.1:8080`).
/// 3.  Attempts to establish a TCP connection to the server.
///     - Logs success or failure. Exits gracefully on connection failure.
/// 4.  Splits the `TcpStream` into a reader and a writer. The reader is wrapped
///     in a `BufReader` for efficient line-based reading.
/// 5.  Enters the main command loop:
///     a.  Prompts the user to enter a command.
///     b.  Reads the user's input.
///     c.  Handles "exit" or "quit" commands to terminate the loop and disconnect.
///     d.  Skips empty input lines.
///     e.  Sends the command (newline-terminated) to the server.
///         - Logs and breaks the loop on send errors.
///     f.  Enters a nested loop to read the server's response:
///         i.  Reads lines from the server.
///         ii. If `Ok(0)` (EOF) is received, it means the server closed the connection.
///             Logs this and exits the entire application.
///         iii. For `LS` commands, it continues reading lines until an empty line
///              is received (which signifies the end of the `LS` output).
///         iv. For other commands (like `ECHO`), it assumes a single-line response
///             and breaks after the first line.
///         v.  Prints each received response line to the console.
///         vi. Logs and exits the application on read errors.
/// 6.  After the main loop terminates (e.g., via "exit" command), logs that the
///     client is shutting down.
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>`:
///   - `Ok(())` if the client runs and terminates gracefully (either by user command
///     or server disconnect).
///   - `Err` if a critical I/O error occurs (though many errors are handled by
///     graceful exit with `Ok(())` after logging).
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger. Set RUST_LOG environment variable for log level, e.g., RUST_LOG=debug.
    env_logger::init();

    let server_addr = "127.0.0.1:8080";
    info!("Attempting to connect to server at {}...", server_addr);

    // Establish a TCP connection to the server.
    let stream = match TcpStream::connect(server_addr).await {
        Ok(stream) => {
            // Successfully connected. `peer_addr()` should be safe to unwrap here as stream is valid.
            info!("Successfully connected to server: {:?}", stream.peer_addr().unwrap_or_else(|_| "unknown peer".parse().unwrap()));
            stream
        }
        Err(e) => {
            error!("Failed to connect to server {}: {}", server_addr, e);
            // Also print to stderr for visibility if RUST_LOG is not configured.
            eprintln!("Error: Failed to connect to server at {}. Please ensure r_server is running.", server_addr);
            return Ok(()); // Exit gracefully, as this is an expected failure mode.
        }
    };

    // Split the TCP stream into a reader and a writer part.
    // `BufReader` provides buffered reading capabilities.
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    // Main command loop: prompt user, send command, receive response.
    loop {
        // Display prompt to the user.
        print!("Enter command (ECHO <msg>, LS, or 'exit'/'quit'): ");
        // Ensure the prompt is displayed before reading input.
        stdout().flush()?; 

        let mut user_input = String::new();
        // Read a line of input from the user.
        if stdin().read_line(&mut user_input)? == 0 {
            // This case (Ctrl+D on empty line / EOF on stdin) could also trigger a shutdown.
            info!("EOF received on stdin. Shutting down client.");
            break;
        }

        let command = user_input.trim(); // Remove leading/trailing whitespace.

        // Check for client-side exit commands.
        if command == "exit" || command == "quit" {
            info!("'{}' command received. Disconnecting from server.", command);
            break; // Exit the main command loop.
        }

        // Skip sending if the command is empty after trimming.
        if command.is_empty() {
            continue;
        }
        
        debug!("Sending command to server: \"{}\"", command);
        // Send the command to the server, appending a newline as per protocol.
        if let Err(e) = writer.write_all(format!("{}\n", command).as_bytes()).await {
            error!("Error sending command \"{}\" to server: {}", command, e);
            eprintln!("Error: Could not send command to server. Connection may be lost.");
            break; // Assume connection is lost, exit loop.
        }

        // Read and display the server's response.
        debug!("Waiting for server response for command \"{}\"...", command);
        // User-facing message indicating that we are waiting for a response.
        // Log messages provide more detailed insight for debugging.
        println!("Server response:"); 
        
        // Inner loop for reading potentially multi-line responses (especially for LS).
        loop {
            let mut server_response_line = String::new();
            // Read a single line from the server's response.
            match buf_reader.read_line(&mut server_response_line).await {
                Ok(0) => {
                    // `Ok(0)` indicates the server closed the connection (EOF).
                    info!("Server closed the connection while waiting for response.");
                    println!("Server closed the connection.");
                    return Ok(()); // Gracefully exit the application.
                }
                Ok(_) => {
                    // Successfully read a line from the server.
                    let trimmed_response = server_response_line.trim_end_matches('\n');
                    debug!("Received line from server: \"{}\"", trimmed_response);
                    println!("{}", trimmed_response); // Display the line to the user.

                    // Determine if more lines are expected based on the command sent.
                    if command.starts_with("LS") {
                        // For "LS", the server sends an empty line to terminate the list.
                        if trimmed_response.is_empty() {
                            debug!("End of LS response detected (empty line).");
                            break; // Exit the inner response-reading loop.
                        }
                        // Otherwise, continue reading more lines for LS.
                    } else {
                        // For non-LS commands (like ECHO or ERR), assume a single-line response.
                        debug!("Non-LS command (\"{}\"), single line response assumed.", command);
                        break; // Exit the inner response-reading loop.
                    }
                }
                Err(e) => {
                    // An error occurred while reading the response from the server.
                    error!("Error reading response from server for command \"{}\": {}", command, e);
                    eprintln!("Error: Failed to read response from server. Connection may be lost.");
                    return Ok(()); // Gracefully exit the application.
                }
            }
        }
    }

    info!("Client shutting down.");
    Ok(())
}
