# PeerShare - Backend

PeerShare is a peer-to-peer file-sharing service built using Rust and Actix-Web. It enables users to share files across the internet using WebSockets and WebRTC. The backend facilitates session management, room creation, and secure communication between peers.

## Features

- Peer-to-peer file sharing
- WebSocket-based real-time communication
- Secure HTTPS connection with OpenSSL
- Dynamic room creation and management
- Direct messaging between users

## Installation

### Prerequisites

Ensure you have the following installed:
- [Rust](https://www.rust-lang.org/tools/install)
- [OpenSSL](https://www.openssl.org/)
- A TLS certificate and key (`server.crt` and `server.key`)

### Setup

1. Clone the repository:
   ```sh
   git clone https://github.com/yourusername/peershare.git
   cd peershare
   ```

2. Create a `.env` file and define the following environment variables:
   ```sh
   ADDR=0.0.0.0:8080
   QUEUE_LENGHT=10
   ```

3. Build and run the project:
   ```sh
   cargo run --release
   ```

## Usage

### WebSocket Connection
- Clients can connect to `/ws` for WebSocket-based communication.
- Messages can be sent in a structured format for room management and file-sharing.

### Static File Hosting
- The server serves static files from the `./static` directory.
- The default index page is `index.html`.

## Project Structure

```
.
├── src
│   ├── main.rs        # Entry point of the application
│   ├── server.rs      # ChatServer implementation for handling WebSocket connections
│   ├── queue.rs       # Queue management
│   ├── session.rs     # Session handling
│   ├── reserr.rs      # Error handling
│   ├── routes.rs      # WebSocket route handling
│
├── static            # Directory for static frontend files
├── Cargo.toml        # Dependencies and project metadata
├── .env              # Environment variables (ignored in version control)
```

## Dependencies

- `actix-web` - Web framework
- `actix-web-actors` - WebSocket support
- `actix` - Actor framework
- `rand` - Random number generation
- `actix-files` - Static file hosting
- `dotenv` - Environment variable management
- `openssl` - TLS support
