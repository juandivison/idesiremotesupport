# Rust Remote Command Tool

## Description

This project implements a simple client-server application in Rust that allows for basic remote command execution. The server listens for TCP connections, and the client can connect to the server to issue commands such as echoing a message or listing files in the server's current directory.

## Prerequisites

*   **Rust and Cargo:** You need to have Rust and Cargo installed on your system. You can install them from [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).

## Directory Structure

The project is organized as follows:

```
project_root/
├── r_server/  # The server application crate
├── r_client/  # The client application crate
└── README.md  # This file
```

## Building the Applications

To build both the server and client applications, navigate to their respective directories and use `cargo build`.

1.  **Build the Server:**
    ```bash
    cd r_server
    cargo build
    ```

2.  **Build the Client:**
    ```bash
    cd r_client
    cargo build
    ```

For optimized release builds, use the `--release` flag:
```bash
cargo build --release
```
Remember to use the path `target/release/` instead of `target/debug/` when running release builds.

## Running the Server (`r_server`)

1.  **Navigate to the server directory:**
    ```bash
    cd r_server
    ```

2.  **Run the server executable:**
    *   For debug build:
        ```bash
        ./target/debug/r_server
        ```
    *   For release build:
        ```bash
        ./target/release/r_server
        ```

3.  **Server Address:** The server listens on the default address `127.0.0.1:8080`.

4.  **Logging:**
    The server uses `env_logger`. You can control the log level using the `RUST_LOG` environment variable.
    *   To see informational messages:
        ```bash
        RUST_LOG=info ./target/debug/r_server
        ```
    *   To see debug messages specifically from `r_server` (if you have multiple crates):
        ```bash
        RUST_LOG=r_server=debug ./target/debug/r_server
        ```
    *   For more verbose logging, including from dependencies:
        ```bash
        RUST_LOG=debug ./target/debug/r_server
        ```

## Running the Client (`r_client`)

1.  **Ensure the server (`r_server`) is running first.**

2.  **Navigate to the client directory:**
    ```bash
    cd r_client
    ```

3.  **Run the client executable:**
    *   For debug build:
        ```bash
        ./target/debug/r_client
        ```
    *   For release build:
        ```bash
        ./target/release/r_client
        ```

4.  **Connecting to Server:** The client will automatically attempt to connect to the server at `127.0.0.1:8080`.

5.  **Logging:**
    The client also uses `env_logger`. You can control its log level similarly:
    *   To see informational messages:
        ```bash
        RUST_LOG=info ./target/debug/r_client
        ```
    *   For debug messages:
        ```bash
        RUST_LOG=r_client=debug ./target/debug/r_client
        ```

## Basic Usage

Once the client is connected to the server, you will see a prompt. You can then type commands:

*   **`LS`**: Lists files and directories in the server's current working directory. The server sends each entry on a new line, followed by a blank line to indicate the end of the list.
*   **`ECHO <message>`**: The server will echo back the `<message>` you provide.
*   **`exit` or `quit`**: Type either of these commands to disconnect the client and shut it down.

**Example Interaction:**

Client-side:
```
$ ./target/debug/r_client 
# (Assuming r_server is running in another terminal)
# Output from r_client if RUST_LOG=info is set:
# [INFO  r_client] Attempting to connect to server at 127.0.0.1:8080...
# [INFO  r_client] Successfully connected to server: 127.0.0.1:8080
Enter command (ECHO <msg>, LS, or 'exit'/'quit'): LS
Server response:
src
target
Cargo.toml
# (Blank line indicating end of LS)
Enter command (ECHO <msg>, LS, or 'exit'/'quit'): ECHO Hello from client!
Server response:
Hello from client!
Enter command (ECHO <msg>, LS, or 'exit'/'quit'): quit
# Output from r_client if RUST_LOG=info is set:
# [INFO  r_client] 'quit' command received. Disconnecting from server.
# [INFO  r_client] Client shutting down.
```

Server-side (example logs with `RUST_LOG=info,r_server::handler=debug`):
```
[INFO  r_server] Server starting on 127.0.0.1:8080...
[INFO  r_server] Server listening on 127.0.0.1:8080
[INFO  r_server::main] Accepted new connection from: 127.0.0.1:XXXXX # XXXXX is a port number
[INFO  r_server::handler] Handling connection from: 127.0.0.1:XXXXX
[DEBUG r_server::handler] Received raw line from 127.0.0.1:XXXXX: LS
[DEBUG r_server::handler] Processing LS command for 127.0.0.1:XXXXX
[DEBUG r_server::handler] LS command processed successfully for 127.0.0.1:XXXXX
[DEBUG r_server::handler] Received raw line from 127.0.0.1:XXXXX: ECHO Hello from client!
[DEBUG r_server::handler] Echoing to 127.0.0.1:XXXXX: Hello from client!
[INFO  r_server::handler] Client 127.0.0.1:XXXXX disconnected (EOF)
[INFO  r_server::handler] Closing connection from 127.0.0.1:XXXXX
```
(Note: Actual file listings for `LS` will depend on the contents of the `r_server` directory when it's run. Port numbers `XXXXX` will vary.)

```
