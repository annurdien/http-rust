use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::Path;
use std::fs;
use mime_guess::from_path;
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const HEADER_PACKET_LENGTH: usize = 8192; // Increased buffer size for larger headers
const SERVER_NAME: &str = "HTTP-Rust/1.0";

type AllowedFileTable = Arc<RwLock<Vec<String>>>;

#[derive(Debug, Clone)]
struct HttpRequest {
    method: String,
    path: String,
    version: String,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpRequest {
    fn new() -> Self {
        HttpRequest {
            method: String::new(),
            path: String::new(),
            version: String::new(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }
    
    fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_lowercase())
    }
}

#[derive(Debug)]
struct HttpResponse {
    status_code: u16,
    status_message: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpResponse {
    fn new(status_code: u16) -> Self {
        let status_message = get_status_message(status_code);
        let mut response = HttpResponse {
            status_code,
            status_message,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        
        // Add required headers
        response.add_header("Date", &get_current_date());
        response.add_header("Server", SERVER_NAME);
        response.add_header("Connection", "keep-alive");
        
        // For error responses, set empty body and Content-Length: 0
        if status_code >= 400 {
            response.add_header("Content-Length", "0");
        }
        
        response
    }
    
    fn add_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }
    
    fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
        self.add_header("Content-Length", &self.body.len().to_string());
    }
    
    fn set_chunked_body(&mut self, body: Vec<u8>) {
        self.body = body;
        self.add_header("Transfer-Encoding", "chunked");
        // Remove Content-Length for chunked encoding
        self.headers.remove("Content-Length");
    }
    
    fn to_bytes(&self, include_body: bool) -> Vec<u8> {
        let mut response = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_message);
        
        for (key, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }
        
        response.push_str("\r\n");
        
        let mut bytes = response.into_bytes();
        if include_body {
            if self.headers.get("Transfer-Encoding").map_or(false, |v| v == "chunked") {
                // Add chunked encoding format
                let chunk_size = format!("{:x}\r\n", self.body.len());
                bytes.extend(chunk_size.as_bytes());
                bytes.extend(&self.body);
                bytes.extend(b"\r\n0\r\n\r\n"); // End chunk
            } else {
                bytes.extend(&self.body);
            }
        }
        bytes
    }
}

fn get_status_message(code: u16) -> String {
    match code {
        200 => "OK".to_string(),
        201 => "Created".to_string(),
        204 => "No Content".to_string(),
        400 => "Bad Request".to_string(),
        403 => "Forbidden".to_string(),
        404 => "Not Found".to_string(),
        405 => "Method Not Allowed".to_string(),
        411 => "Length Required".to_string(),
        413 => "Payload Too Large".to_string(),
        500 => "Internal Server Error".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn get_current_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    
    // RFC 1123 date format
    let timestamp = now.as_secs();
    let days_since_epoch = timestamp / 86400;
    let seconds_in_day = timestamp % 86400;
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;
    let seconds = seconds_in_day % 60;
    
    // Simple approximation - this could be improved with a proper date library
    let weekday = match (days_since_epoch + 4) % 7 { // Jan 1, 1970 was Thursday
        0 => "Sun", 1 => "Mon", 2 => "Tue", 3 => "Wed", 
        4 => "Thu", 5 => "Fri", 6 => "Sat", _ => "Thu"
    };
    
    // Very basic date calculation (approximation)
    let year = 1970 + (days_since_epoch / 365);
    let day_of_month = 1 + (days_since_epoch % 31);
    
    format!("{}, {:02} Jan {} {:02}:{:02}:{:02} GMT", 
             weekday, day_of_month, year, hours, minutes, seconds)
}

#[tokio::main]
async fn main() {
    let allowed_file_table = Arc::new(RwLock::new(create_allowed_file_table()));

    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
    println!("Server listening on port 7878");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let allowed_file_table = Arc::clone(&allowed_file_table);
        tokio::spawn(async move {
            handle_client(socket, allowed_file_table).await;
        });
    }
}

async fn handle_client(mut socket: TcpStream, allowed_file_table: AllowedFileTable) {
    loop {
        let mut buffer = [0; HEADER_PACKET_LENGTH];
        
        match socket.read(&mut buffer).await {
            Ok(n) if n == 0 => break, // Connection closed by client
            Ok(n) => {
                let request_data = String::from_utf8_lossy(&buffer[..n]);
                
                if let Some(request) = parse_http_request(&request_data) {
                    let response = process_request(request.clone(), &allowed_file_table).await;
                    let include_body = request.method != "HEAD";
                    
                    if let Err(_) = socket.write_all(&response.to_bytes(include_body)).await {
                        break; // Write error, close connection
                    }
                    
                    // Check if client wants to close connection
                    if let Some(connection_header) = request.get_header("connection") {
                        if connection_header.to_lowercase() == "close" {
                            break;
                        }
                    }
                    
                    // Check if we should close connection (HTTP/1.0 default behavior)
                    if request.version == "HTTP/1.0" && request.get_header("connection").map_or(true, |v| v.to_lowercase() != "keep-alive") {
                        break;
                    }
                } else {
                    // Send bad request and close connection
                    let response = HttpResponse::new(400);
                    let _ = socket.write_all(&response.to_bytes(true)).await;
                    break;
                }
            }
            Err(_) => break, // Read error, close connection
        }
    }
}

fn parse_http_request(request_data: &str) -> Option<HttpRequest> {
    let mut lines = request_data.lines();
    let mut request = HttpRequest::new();
    
    // Parse request line
    if let Some(first_line) = lines.next() {
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() != 3 {
            return None;
        }
        
        request.method = parts[0].to_uppercase();
        request.path = parts[1].to_string();
        request.version = parts[2].to_string();
    } else {
        return None;
    }
    
    // Parse headers (case-insensitive)
    for line in lines {
        if line.trim().is_empty() {
            break; // End of headers
        }
        
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_lowercase();
            let value = line[colon_pos + 1..].trim().to_string();
            request.headers.insert(key, value);
        }
    }
    
    Some(request)
}

async fn process_request(request: HttpRequest, allowed_file_table: &AllowedFileTable) -> HttpResponse {
    match request.method.as_str() {
        "GET" => handle_get_request(&request, allowed_file_table).await,
        "HEAD" => handle_head_request(&request, allowed_file_table).await,
        "POST" => handle_post_request(&request, allowed_file_table).await,
        "PUT" => handle_put_request(&request, allowed_file_table).await,
        "DELETE" => handle_delete_request(&request, allowed_file_table).await,
        "OPTIONS" => handle_options_request(&request).await,
        _ => {
            let mut response = HttpResponse::new(405);
            response.add_header("Allow", "GET, HEAD, POST, PUT, DELETE, OPTIONS");
            response
        }
    }
}

async fn handle_get_request(request: &HttpRequest, allowed_file_table: &AllowedFileTable) -> HttpResponse {
    let path = request.path.trim_start_matches('/');
    
    // Special case for root path
    let path = if path.is_empty() { "index.html" } else { path };
    
    let allowed = {
        let table = allowed_file_table.read().await;
        table.iter().find(|entry| entry.ends_with(path)).cloned()
    };

    if let Some(full_path) = allowed {
        if let Ok(content) = fs::read(&full_path) {
            let content_type = from_path(&full_path).first_or_octet_stream();
            let mut response = HttpResponse::new(200);
            response.add_header("Content-Type", &content_type.to_string());
            response.add_header("Access-Control-Allow-Origin", "*");
            
            // Use chunked encoding for larger files (>1KB) if client supports HTTP/1.1
            if content.len() > 1024 && request.version == "HTTP/1.1" {
                response.set_chunked_body(content);
            } else {
                response.set_body(content);
            }
            
            response
        } else {
            HttpResponse::new(404) // File in allowed list but doesn't exist
        }
    } else {
        // Check if file exists but is not allowed
        let file_path = format!("./public/{}", path);
        if Path::new(&file_path).exists() {
            HttpResponse::new(403) // File exists but forbidden
        } else {
            HttpResponse::new(404) // File doesn't exist
        }
    }
}

async fn handle_head_request(request: &HttpRequest, allowed_file_table: &AllowedFileTable) -> HttpResponse {
    // HEAD is like GET but without body
    handle_get_request(request, allowed_file_table).await
    // Note: body exclusion is handled in handle_client
}

async fn handle_post_request(_request: &HttpRequest, _allowed_file_table: &AllowedFileTable) -> HttpResponse {
    // Basic POST handling - just return 201 Created for now
    let mut response = HttpResponse::new(201);
    response.add_header("Content-Type", "application/json");
    response.set_body(b"{\"message\": \"POST request received\", \"status\": \"success\"}".to_vec());
    response
}

async fn handle_put_request(_request: &HttpRequest, _allowed_file_table: &AllowedFileTable) -> HttpResponse {
    // Basic PUT handling - return 204 No Content
    let mut response = HttpResponse::new(204);
    response.add_header("Allow", "GET, HEAD, POST, PUT, DELETE, OPTIONS");
    response
}

async fn handle_delete_request(_request: &HttpRequest, _allowed_file_table: &AllowedFileTable) -> HttpResponse {
    // Basic DELETE handling - return 204 No Content
    let mut response = HttpResponse::new(204);
    response.add_header("Allow", "GET, HEAD, POST, PUT, DELETE, OPTIONS");
    response
}

async fn handle_options_request(_request: &HttpRequest) -> HttpResponse {
    let mut response = HttpResponse::new(200);
    response.add_header("Allow", "GET, HEAD, POST, PUT, DELETE, OPTIONS");
    response.add_header("Access-Control-Allow-Origin", "*");
    response.add_header("Access-Control-Allow-Methods", "GET, HEAD, POST, PUT, DELETE, OPTIONS");
    response.add_header("Access-Control-Allow-Headers", "Content-Type, Authorization");
    response
}

async fn send_forbidden_packet(socket: &mut TcpStream) {
    let response = HttpResponse::new(403);
    let _ = socket.write_all(&response.to_bytes(true)).await;
}

async fn send_bad_request_packet(socket: &mut TcpStream) {
    let response = HttpResponse::new(400);
    let _ = socket.write_all(&response.to_bytes(true)).await;
}

async fn send_content(path: &str, socket: &mut TcpStream) {
    if let Ok(content) = fs::read(path) {
        let content_type = from_path(path).first_or_octet_stream();
        let mut response = HttpResponse::new(200);
        response.add_header("Content-Type", &content_type.to_string());
        response.add_header("Access-Control-Allow-Origin", "*");
        response.set_body(content);
        let _ = socket.write_all(&response.to_bytes(true)).await;
    } else {
        send_forbidden_packet(socket).await;
    }
}

fn create_allowed_file_table() -> Vec<String> {
    let paths = vec!["./public/index.html", "./public/style.css", "./public/large.html"];
    let mut table = Vec::new();
    for path in paths {
        if Path::new(path).exists() {
            table.push(path.to_string());
        }
    }
    table
}
