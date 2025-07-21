# HTTP Rust Server

A lightweight, asynchronous HTTP/1.1 server implementation written in Rust using Tokio. This project demonstrates modern HTTP server architecture with support for persistent connections, chunked encoding, and comprehensive HTTP/1.1 compliance.

## Features

### Core HTTP/1.1 Support
- **Multiple HTTP Methods**: GET, POST, HEAD, and more
- **Persistent Connections**: Keep-alive support for improved performance
- **Chunked Transfer Encoding**: Efficient handling of large responses
- **Case-Insensitive Headers**: RFC-compliant header processing
- **Multi-Field Headers**: Support for headers with multiple values
- **Comprehensive Status Codes**: Proper error handling and response codes

### Server Features
- **Asynchronous Architecture**: Built on Tokio for high concurrency
- **Static File Serving**: Efficient serving of static content
- **MIME Type Detection**: Automatic content-type detection
- **CORS Support**: Cross-Origin Resource Sharing headers
- **Security**: File access control and path validation

### RFC Compliance
- Required headers (Date, Server, Connection)
- Proper request parsing and response formatting
- HTTP/1.0 and HTTP/1.1 protocol support

## Quick Start

### Prerequisites
- Rust 1.70 or later
- Cargo package manager

### Installation and Running

1. Clone the repository:
```bash
git clone https://github.com/annurdien/http-rust.git
cd http-rust
```

2. Build and run the server:
```bash
cargo run
```

3. Test the server:
```bash
# Basic GET request
curl http://127.0.0.1:7878/index.html

# With verbose output to see headers
curl -v http://127.0.0.1:7878/index.html

# POST request example
curl -X POST -d "test data" http://127.0.0.1:7878/
```

The server will start on `127.0.0.1:7878` and serve files from the `public/` directory.

## Configuration

The server currently serves files from the `public/` directory. Allowed files are configured in the `create_allowed_file_table()` function in `src/main.rs`.

Default served files:
- `public/index.html` - Main page
- `public/style.css` - Stylesheet

## Architecture

This server implements a modern asynchronous architecture:

- **Async/Await**: Uses Tokio runtime for non-blocking I/O
- **Connection Pooling**: Supports multiple concurrent connections
- **Request Processing**: Efficient parsing and response generation
- **File System**: Secure file serving with access controls

For detailed architecture information, see [docs/architecture.md](docs/architecture.md).

## HTTP/1.1 Compliance

This implementation supports most HTTP/1.1 features as defined in RFC 7230-7235:

- Protocol negotiation and version handling
- Request/response message format
- Header field parsing and generation
- Connection management
- Transfer encoding

For a detailed feature matrix, see [docs/http1-spec.md](docs/http1-spec.md).

## Development

### Building
```bash
cargo build
```

### Checking for errors
```bash
cargo check
```

### Running tests
```bash
cargo test
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

This project is open source. See the repository for license details.

## Additional Documentation

- [Architecture Details](docs/architecture.md) - Server design and components
- [HTTP/1.1 Specification Compliance](docs/http1-spec.md) - Feature support matrix