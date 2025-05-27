//! # Client Handler Logic for R-Server
//!
//! This module is responsible for handling individual client connections to the R-Server.
//! It includes parsing client commands, processing those commands (e.g., echoing
//! messages, listing directory contents), and sending responses back to the client.
//!
//! The main entry point is the `handle_client` asynchronous function, which manages
//! the lifecycle of a single client connection. It utilizes helper functions to
//! parse commands and prepare appropriate responses.

use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use log::{info, warn, error, debug};
use std::net::SocketAddr;
use std::path::Path;

// --- Command Processing Logic & Types ---

/// Represents the commands a client can send to the server.
#[derive(Debug, PartialEq)]
pub enum Command {
    /// The `ECHO` command. The associated `String` contains the message to be echoed.
    Echo(String),
    /// The `LS` command, requesting a list of files/directories.
    Ls,
    /// Represents an unrecognized or unsupported command.
    Unknown,
}

/// Represents an error that occurred during command parsing.
///
/// Currently, this is a simple wrapper around a `String` message, but it could
/// be expanded to include more structured error information (e.g., error codes).
#[derive(Debug, PartialEq)]
pub struct ParseError(String); // Simple error type for now

/// Parses a raw input line from a client into a `Command`.
///
/// # Arguments
///
/// * `line`: A string slice (`&str`) representing the raw line received from the client.
///           This line is expected to be newline-terminated, but the function trims
///           whitespace before parsing.
///
/// # Returns
///
/// * `Result<Command, ParseError>`:
///   - `Ok(Command)`: If the line is successfully parsed into a known command.
///     - `Command::Echo(message)` for "ECHO <message>" or "ECHO" (empty message).
///     - `Command::Ls` for "LS".
///     - `Command::Unknown` for any other input.
///   - `Err(ParseError)`: If parsing fails (though currently, all unknown commands
///                        are mapped to `Command::Unknown` rather than an error).
///
/// # Behavior
///
/// - Trims leading/trailing whitespace from the input `line`.
/// - Case-sensitive: "ECHO" is recognized, but "echo" would be `Command::Unknown`.
/// - "ECHO" (with no argument) is treated as `Command::Echo("")`.
pub fn parse_command(line: &str) -> Result<Command, ParseError> {
    let trimmed_line = line.trim(); // Remove leading/trailing whitespace
    if trimmed_line.starts_with("ECHO ") {
        Ok(Command::Echo(trimmed_line[5..].to_string()))
    } else if trimmed_line == "ECHO" { // Handle "ECHO" without a message as an empty echo
        Ok(Command::Echo("".to_string()))
    } else if trimmed_line == "LS" {
        Ok(Command::Ls)
    } else {
        // For simplicity, any unrecognized command is mapped to Command::Unknown.
        // Alternatively, this could return Err(ParseError("Unknown command".to_string())).
        Ok(Command::Unknown)
    }
}

/// Prepares the response string for an `ECHO` command.
///
/// # Arguments
///
/// * `message`: The string slice (`&str`) to be echoed.
///
/// # Returns
///
/// A `String` containing the `message` followed by a newline character (`\n`).
/// This is the format expected by the client.
pub fn prepare_echo_response(message: &str) -> String {
    format!("{}\n", message) // Append newline for client protocol
}

/// Prepares the response lines for an `LS` command by listing entries in the specified directory.
///
/// # Arguments
///
/// * `current_dir`: A `&Path` representing the directory whose contents are to be listed.
///                  Typically, this will be the server's current working directory (`.`).
///
/// # Returns
///
/// * `Result<Vec<String>, std::io::Error>`:
///   - `Ok(Vec<String>)`: A vector where each `String` is a file or directory name
///                        within `current_dir`. The names do not include the path.
///   - `Err(std::io::Error)`: If an I/O error occurs while reading the directory (e.g.,
///                            the directory does not exist or permissions are denied).
///
/// # Note
///
/// The order of entries is determined by the underlying `std::fs::read_dir` iterator
/// and is not guaranteed to be sorted.
pub fn prepare_ls_response(current_dir: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut entries = Vec::new();
    for entry_result in std::fs::read_dir(current_dir)? {
        let entry = entry_result?; // Propagate I/O errors for individual entries
        entries.push(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(entries)
}

/// Prepares a generic error response string for the client.
///
/// # Arguments
///
/// * `message`: A string slice (`&str`) describing the error.
///
/// # Returns
///
/// A `String` formatted as "ERR <message>\n". This is the standard error format
/// expected by the client.
pub fn prepare_error_response(message: &str) -> String {
    format!("ERR {}\n", message) // Standard error prefix + newline
}
// --- End of Command Processing Logic ---

/// Handles an individual client connection.
///
/// This asynchronous function is responsible for the entire lifecycle of a client
/// interaction after the TCP connection has been established. It reads commands
/// from the client, processes them, and sends back responses.
///
/// # Arguments
///
/// * `stream`: A `tokio::net::TcpStream` representing the connected client socket.
///
/// # Behavior
///
/// 1.  Retrieves and logs the client's peer address. If `peer_addr()` fails (which is
///     unlikely for an accepted stream), it defaults to a placeholder and logs the event.
/// 2.  Wraps the `TcpStream` in a `BufReader` for efficient, line-buffered reading.
/// 3.  Enters a loop to continuously read lines (commands) from the client:
///     a.  If `read_line` returns `Ok(0)`, it means the client has closed the connection (EOF).
///         The loop breaks, and the connection is closed.
///     b.  If a line is successfully read:
///         i.  It's logged at the `debug` level.
///         ii. The line is parsed using `parse_command`.
///         iii. Based on the parsed `Command`:
///             -   `Command::Echo(msg)`: `prepare_echo_response` is called, and the result is sent.
///             -   `Command::Ls`: `prepare_ls_response` is called (for the current directory ".").
///                 Each directory entry is sent as a separate line, followed by a blank line
///                 to signify the end of the LS output. If `prepare_ls_response` fails
///                 (e.g., directory read error), `prepare_error_response` is used.
///             -   `Command::Unknown` (or `Err` from `parse_command`):
///                 `prepare_error_response("Unknown command")` is sent.
///         iv. Any error during writing to the socket causes the loop to break, effectively
///             terminating the connection for that client.
///     c.  If `read_line` itself returns an error, the error is logged, and the loop breaks.
///         Connection errors are distinguished from clean EOFs.
/// 4.  After the loop terminates (either by client disconnect, error, or other means),
///     a final log message indicates the connection is closing.
///
/// # Error Handling
///
/// - Socket read/write errors are logged, and generally lead to termination of the handler for that client.
/// - Directory reading errors for `LS` are reported to the client as an "ERR" message.
/// - Unknown commands are reported to the client as an "ERR" message.
pub async fn handle_client(stream: TcpStream) {
    // Attempt to get client's address, with a fallback for safety (though peer_addr on accepted stream should be reliable)
    let client_addr = stream.peer_addr().unwrap_or_else(|e| {
        error!("Failed to get peer address: {}. Using placeholder.", e);
        // SocketAddr::from_str("0.0.0.0:0").unwrap() would be an option if a valid SocketAddr is strictly needed
        // For logging purposes, a string might be sufficient if this case is truly exceptional.
        "unknown_client".parse::<SocketAddr>().expect("Failed to parse placeholder SocketAddr. This should not happen.")
    });
    info!("Handling connection from: {}", client_addr);

    // Wrap the stream in a BufReader for efficient line-by-line reading.
    let mut reader = BufReader::new(stream);
    let mut line = String::new(); // Buffer for reading lines

    loop {
        line.clear(); // Clear buffer for the next read
        // Read a line from the client. A line is terminated by '\n'.
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // `Ok(0)` means EOF was reached, indicating the client closed the connection.
                info!("Client {} disconnected (EOF)", client_addr);
                break; // Exit the loop, which will lead to closing the connection.
            }
            Ok(_) => {
                // Successfully read a line.
                // Log the raw command received (excluding the newline for cleaner logs).
                debug!("Received raw line from {}: {}", client_addr, line.trim_end_matches('\n'));

                // Parse the command from the received line.
                match parse_command(&line) {
                    Ok(Command::Echo(msg)) => {
                        // Prepare and send the ECHO response.
                        let response = prepare_echo_response(&msg);
                        debug!("Echoing to {}: {}", client_addr, msg);
                        if let Err(e) = reader.get_mut().write_all(response.as_bytes()).await {
                            error!("Error writing ECHO response to socket for {}: {}", client_addr, e);
                            break; // Error writing to socket, assume connection is broken.
                        }
                    }
                    Ok(Command::Ls) => {
                        // Process the LS command.
                        debug!("Processing LS command for {}", client_addr);
                        // Use the current directory "." for listing.
                        match prepare_ls_response(Path::new(".")) {
                            Ok(entries) => {
                                // Send each directory entry as a separate line.
                                for file_name in entries {
                                    let response_line = format!("{}\n", file_name);
                                    if let Err(e) = reader.get_mut().write_all(response_line.as_bytes()).await {
                                        error!("Error writing LS item to socket for {}: {}", client_addr, e);
                                        // Critical error: if we can't write a part of the LS response,
                                        // it's best to terminate this client's handler.
                                        return;
                                    }
                                }
                                // After sending all entries, send a blank line to signify the end of LS output.
                                if let Err(e) = reader.get_mut().write_all(b"\n").await {
                                    error!("Error writing LS terminator to socket for {}: {}", client_addr, e);
                                    break; // Error writing, assume connection is broken.
                                }
                                debug!("LS command processed successfully for {}", client_addr);
                            }
                            Err(e) => {
                                // An error occurred reading the directory (e.g., permissions).
                                error!("Error reading directory for LS command for {}: {}", client_addr, e);
                                let response = prepare_error_response("Error reading directory");
                                if let Err(e_write) = reader.get_mut().write_all(response.as_bytes()).await {
                                    error!("Error writing LS error response to socket for {}: {}", client_addr, e_write);
                                    break; // Error writing, assume connection is broken.
                                }
                            }
                        }
                    }
                    Ok(Command::Unknown) | Err(_) => {
                        // Handles Command::Unknown or any ParseError.
                        // Log the unrecognized command and send an error response to the client.
                        warn!("Client {} sent unknown or malformed command: {}", client_addr, line.trim());
                        let response = prepare_error_response("Unknown command");
                        if let Err(e) = reader.get_mut().write_all(response.as_bytes()).await {
                            error!("Error writing unknown command error to socket for {}: {}", client_addr, e);
                            break; // Error writing, assume connection is broken.
                        }
                    }
                }
            }
            Err(e) => {
                // An error occurred while reading from the socket.
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    // This can happen if the client disconnects abruptly without a clean shutdown.
                    info!("Client {} disconnected (Unexpected EOF while reading)", client_addr);
                } else {
                    // Other types of I/O errors.
                    error!("Error reading from socket for client {}: {}", client_addr, e);
                }
                break; // Exit the loop on any read error.
            }
        }
    }
    // Log that the connection handling is ending for this client.
    info!("Closing connection from {}", client_addr);
}

// Test module remains unchanged, so it's omitted for brevity in this diff.
// Ensure existing tests are reviewed for clarity and coverage if necessary.
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_parse_echo_normal() {
        assert_eq!(
            parse_command("ECHO hello world"),
            Ok(Command::Echo("hello world".to_string()))
        );
    }

    #[test]
    fn test_parse_echo_empty() {
        assert_eq!(
            parse_command("ECHO"),
            Ok(Command::Echo("".to_string()))
        );
    }
    
    #[test]
    fn test_parse_echo_with_trailing_whitespace() {
        assert_eq!(
            parse_command("ECHO hello world   "),
            Ok(Command::Echo("hello world".to_string()))
        );
    }

    #[test]
    fn test_parse_ls() {
        assert_eq!(parse_command("LS"), Ok(Command::Ls));
    }

    #[test]
    fn test_parse_ls_with_trailing_whitespace() {
        assert_eq!(parse_command("LS   "), Ok(Command::Ls));
    }

    #[test]
    fn test_parse_unknown() {
        assert_eq!(parse_command("FOOBAR"), Ok(Command::Unknown));
    }
    
    #[test]
    fn test_parse_unknown_with_prefix_echo() {
        assert_eq!(parse_command("ECHOFOO"), Ok(Command::Unknown));
    }


    #[test]
    fn test_prepare_echo_response_normal() {
        assert_eq!(prepare_echo_response("hello world"), "hello world\n");
    }

    #[test]
    fn test_prepare_echo_response_empty() {
        assert_eq!(prepare_echo_response(""), "\n");
    }

    #[test]
    fn test_prepare_ls_response_empty_dir() {
        let dir = tempdir().unwrap();
        let entries = prepare_ls_response(dir.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_prepare_ls_response_with_files() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("file1.txt")).unwrap().write_all(b"hello").unwrap();
        File::create(dir.path().join("file2.txt")).unwrap().write_all(b"world").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let mut entries = prepare_ls_response(dir.path()).unwrap();
        entries.sort(); // Sort for consistent test results

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0], "file1.txt");
        assert_eq!(entries[1], "file2.txt");
        assert_eq!(entries[2], "subdir");
    }
    
    #[test]
    fn test_prepare_ls_response_non_existent_dir() {
        let path = Path::new("/path/that/does/not/exist_hopefully");
        assert!(prepare_ls_response(path).is_err());
    }

    #[test]
    fn test_prepare_error_response() {
        assert_eq!(prepare_error_response("Test error"), "ERR Test error\n");
    }
}
