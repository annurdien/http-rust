# Server Architecture

This document describes the architecture and design of the HTTP Rust server, detailing how the components work together to provide HTTP/1.1 compliance and high performance.

## Overview

The HTTP Rust server is built using Rust's Tokio async runtime, providing a modern, efficient, and scalable HTTP server implementation. The architecture follows async/await patterns for non-blocking I/O operations.

## Core Components

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Rust Server                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   TCP       │  │   Request   │  │  Response   │        │
│  │  Listener   │  │   Parser    │  │ Generator   │        │
│  │             │  │             │  │             │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
│         │                 │                 │             │
│         v                 v                 v             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ Connection  │  │   HTTP      │  │   File      │        │
│  │  Handler    │  │  Protocol   │  │  System     │        │
│  │             │  │  Engine     │  │  Manager    │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
│         │                 │                 │             │
│         └─────────────────┼─────────────────┘             │
│                           │                               │
│                    ┌─────────────┐                        │
│                    │  Security   │                        │
│                    │  & Access   │                        │
│                    │  Control    │                        │
│                    └─────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

### Component Details

| Component | Description | Responsibilities |
|-----------|-------------|------------------|
| **TCP Listener** | Accepts incoming connections | - Bind to port 7878<br>- Accept new connections<br>- Spawn connection handlers |
| **Connection Handler** | Manages individual client connections | - Read request data<br>- Manage connection lifecycle<br>- Handle keep-alive connections |
| **Request Parser** | Parses HTTP requests | - Parse request line<br>- Extract headers<br>- Validate HTTP version<br>- Handle different methods |
| **HTTP Protocol Engine** | Core HTTP/1.1 logic | - Method dispatch<br>- Header processing<br>- Status code generation<br>- Protocol compliance |
| **Response Generator** | Creates HTTP responses | - Generate response headers<br>- Set proper status codes<br>- Handle content encoding<br>- Manage connection headers |
| **File System Manager** | Handles file operations | - Secure file access<br>- MIME type detection<br>- Content serving<br>- Path validation |
| **Security & Access Control** | Enforces security policies | - Allowed file validation<br>- Path traversal protection<br>- Access control lists |

## Request Processing Flow

### Request Lifecycle

```
Client Request
      │
      v
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Accept    │───▶│    Read     │───▶│    Parse    │
│ Connection  │    │   Request   │    │   Headers   │
└─────────────┘    └─────────────┘    └─────────────┘
      │                   │                   │
      v                   v                   v
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Validate   │───▶│   Process   │───▶│  Generate   │
│   Request   │    │   Method    │    │  Response   │
└─────────────┘    └─────────────┘    └─────────────┘
      │                   │                   │
      v                   v                   v
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│    Send     │───▶│   Handle    │───▶│    Close    │
│  Response   │    │ Keep-Alive  │    │ or Reuse    │
└─────────────┘    └─────────────┘    └─────────────┘
```

### Processing Steps

| Step | Description | Time Complexity |
|------|-------------|-----------------|
| 1. **Connection Accept** | TCP listener accepts new connection | O(1) |
| 2. **Request Reading** | Read request data into buffer | O(n) where n = request size |
| 3. **Header Parsing** | Parse HTTP request line and headers | O(m) where m = header count |
| 4. **Request Validation** | Validate method, path, and headers | O(1) |
| 5. **Method Processing** | Handle GET, POST, HEAD, etc. | O(1) |
| 6. **File Access** | Check permissions and read file | O(log k) where k = allowed files |
| 7. **Response Generation** | Create HTTP response with headers | O(1) |
| 8. **Data Transmission** | Send response to client | O(n) where n = response size |

## HTTP/1.1 Protocol Implementation

### Connection Management

```
┌─────────────────────────────────────────────────────────────┐
│                   Connection Lifecycle                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  New Connection                                             │
│         │                                                   │
│         v                                                   │
│  ┌─────────────┐                                           │
│  │   Parse     │                                           │
│  │ Connection  │                                           │
│  │   Header    │                                           │
│  └─────────────┘                                           │
│         │                                                   │
│         v                                                   │
│  ┌─────────────┐      ┌─────────────┐                     │
│  │ Keep-Alive? │─NO──▶│    Close    │                     │
│  │             │      │ Connection  │                     │
│  └─────────────┘      └─────────────┘                     │
│         │ YES                                               │
│         v                                                   │
│  ┌─────────────┐                                           │
│  │   Process   │◀──┐                                       │
│  │   Request   │   │                                       │
│  └─────────────┘   │                                       │
│         │           │                                       │
│         v           │                                       │
│  ┌─────────────┐   │                                       │
│  │    Send     │   │                                       │
│  │  Response   │   │                                       │
│  └─────────────┘   │                                       │
│         │           │                                       │
│         v           │                                       │
│  ┌─────────────┐   │                                       │
│  │   More      │───┘                                       │
│  │ Requests?   │                                           │
│  └─────────────┘                                           │
│         │ NO                                                │
│         v                                                   │
│  ┌─────────────┐                                           │
│  │    Close    │                                           │
│  │ Connection  │                                           │
│  └─────────────┘                                           │
└─────────────────────────────────────────────────────────────┘
```

### Header Processing

| Feature | Implementation | Standard Compliance |
|---------|----------------|-------------------|
| **Case Insensitive** | Convert to lowercase for comparison | RFC 7230 Section 3.2 |
| **Multi-Value Headers** | Split on comma, trim whitespace | RFC 7230 Section 3.2.6 |
| **Required Headers** | Auto-generate Date, Server, Connection | RFC 7231 Section 7 |
| **Content-Length** | Calculate and set for all responses | RFC 7230 Section 3.3.2 |
| **Content-Type** | MIME type detection based on file extension | RFC 7231 Section 3.1.1.5 |
| **Connection** | Manage keep-alive and close directives | RFC 7230 Section 6.1 |

## Performance Characteristics

### Async Architecture Benefits

| Aspect | Synchronous | Asynchronous (Current) |
|--------|-------------|------------------------|
| **Memory Usage** | Thread per connection (~8MB/thread) | Single thread, task per connection (~KB/task) |
| **Scalability** | Limited by thread count | Limited by memory and file descriptors |
| **Context Switching** | Expensive OS thread switches | Lightweight task switches |
| **I/O Blocking** | Blocks entire thread | Non-blocking, other tasks continue |
| **Resource Efficiency** | High overhead | Low overhead |

### Concurrency Model

```
┌─────────────────────────────────────────────────────────────┐
│                    Tokio Runtime                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Main Task                     Worker Tasks                 │
│  ┌─────────────┐              ┌─────────────┐               │
│  │    TCP      │              │  Connection │               │
│  │  Listener   │──────────────▶│   Handler   │               │
│  │             │              │     #1      │               │
│  └─────────────┘              └─────────────┘               │
│         │                              │                    │
│         │                     ┌─────────────┐               │
│         │                     │  Connection │               │
│         └─────────────────────▶│   Handler   │               │
│                               │     #2      │               │
│                               └─────────────┘               │
│                                       │                     │
│                               ┌─────────────┐               │
│                               │  Connection │               │
│                               │   Handler   │               │
│                               │     #N      │               │
│                               └─────────────┘               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Security Model

### File Access Control

The server implements a whitelist-based security model:

```rust
// Simplified security flow
fn is_file_allowed(requested_path: &str, allowed_table: &[String]) -> bool {
    allowed_table.iter()
        .any(|allowed_path| allowed_path.ends_with(requested_path))
}
```

### Security Features

| Feature | Implementation | Purpose |
|---------|----------------|---------|
| **Path Validation** | Whitelist of allowed files | Prevent directory traversal |
| **Access Control** | File table lookup | Restrict served content |
| **Request Size Limits** | Buffer size constraints | Prevent memory exhaustion |
| **Method Restrictions** | Validate HTTP methods | Control allowed operations |

## Error Handling

### Error Response Matrix

| Error Type | HTTP Status | Response Headers | Body |
|------------|-------------|------------------|------|
| **File Not Found** | 404 Not Found | Content-Type: text/html | Empty |
| **Forbidden Access** | 403 Forbidden | Content-Type: text/html | Empty |
| **Bad Request** | 400 Bad Request | Content-Type: text/html | Empty |
| **Method Not Allowed** | 405 Method Not Allowed | Allow: GET, POST, HEAD | Empty |
| **Internal Error** | 500 Internal Server Error | Content-Type: text/html | Empty |

## Future Enhancements

### Planned Improvements

| Enhancement | Priority | Complexity | Impact |
|-------------|----------|------------|--------|
| **HTTPS Support** | High | Medium | Security |
| **Compression** | Medium | Low | Performance |
| **Caching** | Medium | Medium | Performance |
| **Virtual Hosts** | Low | High | Features |
| **WebSocket Support** | Low | High | Features |
| **HTTP/2** | Low | Very High | Protocol |

This architecture provides a solid foundation for a modern HTTP server while maintaining simplicity and performance.