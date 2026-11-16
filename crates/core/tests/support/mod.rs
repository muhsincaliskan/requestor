use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct TestServer {
    pub address: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl TestServer {
    pub fn spawn(respond: impl Fn(&str) -> Vec<u8> + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let respond = Arc::new(respond);
        let recorded = Arc::clone(&requests);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (respond, recorded) = (Arc::clone(&respond), Arc::clone(&recorded));
                thread::spawn(move || serve(stream, respond.as_ref(), &recorded));
            }
        });
        TestServer { address, requests }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }

    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

pub fn http_response(status_line: &str, headers: &[&str], body: &[u8]) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {status_line}\r\n").into_bytes();
    for header in headers {
        response.extend_from_slice(format!("{header}\r\n").as_bytes());
    }
    response.extend_from_slice(
        format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .as_bytes(),
    );
    response.extend_from_slice(body);
    response
}

fn serve(
    mut stream: std::net::TcpStream,
    respond: &(dyn Fn(&str) -> Vec<u8> + Send + Sync),
    recorded: &Mutex<Vec<String>>,
) {
    let request = read_request(&mut stream);
    recorded.lock().unwrap().push(request.clone());
    let _ = stream.write_all(&respond(&request));
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut data = Vec::new();
    let mut chunk = [0u8; 1024];
    while !request_complete(&data) {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(count) => data.extend_from_slice(&chunk[..count]),
        }
    }
    String::from_utf8_lossy(&data).into_owned()
}

fn request_complete(data: &[u8]) -> bool {
    let text = String::from_utf8_lossy(data);
    let Some((head, body)) = text.split_once("\r\n\r\n") else {
        return false;
    };
    let length = head
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(str::trim)
                .and_then(|value| value.parse::<usize>().ok().map(|_| value.to_string()))
        })
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    body.len() >= length
}
