//! # R-Server - Remote Command Execution Server
//!
//! `r_server` is a simple TCP server that listens for incoming client connections
//! and allows clients to execute a limited set of commands:
//! - `ECHO <message>`: Echoes the provided message back to the client.
//! - `LS`: Lists files and directories in the server's current working directory.
//!
//! The server is built using Tokio for asynchronous I/O and `env_logger` for logging.
//! Each client connection is handled in a separate asynchronous task.

use tokio::net::TcpListener;
mod handler;
use log::{info, error};

/// The main entry point for the R-Server application.
///
/// This function performs the following steps:
/// 1. Initializes the `env_logger` for logging.
/// 2. Defines the server address (`127.0.0.1:8080`).
/// 3. Attempts to bind a `TcpListener` to the specified address.
///    - Logs an error and exits if binding fails.
/// 4. Enters an infinite loop to accept incoming connections:
///    - For each accepted connection, logs the client's address.
///    - Spawns a new asynchronous task using `tokio::spawn` to handle the client
///      connection via the `handler::handle_client` function.
///    - Logs an error if accepting a connection fails, but continues to listen
///      for new connections.
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>`:
///   - `Ok(())` if the server starts successfully (though the server loop is infinite).
///   - `Err` if there's a critical error during startup, such as failing to bind the listener.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger. `RUST_LOG` environment variable can be used to set log level.
    // Example: `RUST_LOG=info cargo run` or `RUST_LOG=debug cargo run`
    env_logger::init();

    let addr = "127.0.0.1:8080";
    info!("Server starting on {}...", addr);

    // Attempt to bind the TCP listener to the address.
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => {
            info!("Server listening on {}", addr);
            listener
        }
        Err(e) => {
            error!("Failed to bind to address {}: {}", addr, e);
            // Return the error, causing the application to terminate.
            return Err(Box::new(e));
        }
    };

    // Main server loop: continuously accept new connections.
    loop {
        match listener.accept().await {
            Ok((stream, client_addr)) => {
                // Connection successful.
                info!("Accepted new connection from: {}", client_addr);
                // Spawn a new asynchronous task to handle this client.
                // The `move` keyword transfers ownership of `stream` to the new task.
                tokio::spawn(async move {
                    handler::handle_client(stream).await;
                });
            }
            Err(e) => {
                // An error occurred while accepting a new connection.
                // Log the error and continue listening for other connections.
                // Depending on the nature of the error, more sophisticated error handling
                // (e.g., exiting on certain fatal errors) might be required in a production server.
                error!("Failed to accept new connection: {}", e);
            }
        }
    }
    // Note: This part of the function is unreachable because the loop is infinite.
    // A real server might have a shutdown mechanism that would allow the loop to terminate.
}
