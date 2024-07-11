use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::Path;
use std::fs;
use mime_guess::from_path;
use tokio::sync::RwLock;
use std::sync::Arc;

const HEADER_PACKET_LENGTH: usize = 1024;

type AllowedFileTable = Arc<RwLock<Vec<String>>>;

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
    let mut buffer = [0; HEADER_PACKET_LENGTH];
    if let Ok(n) = socket.read(&mut buffer).await {
        if n == 0 {
            return;
        }

        let request = String::from_utf8_lossy(&buffer[..n]);
        let mut lines = request.lines();

        if let Some(first_line) = lines.next() {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() == 3 && parts[0] == "GET" {
                let path = parts[1].trim_start_matches('/');
                let allowed = {
                    let table = allowed_file_table.read().await;
                    table.iter().find(|entry| entry.ends_with(path)).cloned()
                };

                if let Some(full_path) = allowed {
                    send_content(&full_path, &mut socket).await;
                } else {
                    send_forbidden_packet(&mut socket).await;
                }
            } else {
                send_bad_request_packet(&mut socket).await;
            }
        }
    }
}

async fn send_forbidden_packet(socket: &mut TcpStream) {
    let data = "HTTP/1.1 403 Forbidden\r\nContent-Type: text/html\r\nContent-Length: 0\r\n\r\n";
    println!("Forbidden request requested");
    socket.write_all(data.as_bytes()).await.unwrap();
}

async fn send_bad_request_packet(socket: &mut TcpStream) {
    let data = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nContent-Length: 0\r\n\r\n";
    println!("Bad request requested");
    socket.write_all(data.as_bytes()).await.unwrap();
}

async fn send_content(path: &str, socket: &mut TcpStream) {
    if let Ok(content) = fs::read(path) {
        let content_type = from_path(path).first_or_octet_stream();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
            content_type, content.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.write_all(&content).await.unwrap();
    } else {
        send_forbidden_packet(socket).await;
    }
}

fn create_allowed_file_table() -> Vec<String> {
    let paths = vec!["./public/index.html", "./public/style.css"];
    let mut table = Vec::new();
    for path in paths {
        if Path::new(path).exists() {
            table.push(path.to_string());
        }
    }
    table
}
