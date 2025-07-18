#[cfg(test)]
mod tests {
    use super::super::*;
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::time::Duration;
    use std::{fs, thread};
    use tempfile::tempdir;

    fn start_test_server(address: &str, root_dir: PathBuf) -> thread::JoinHandle<()> {
        let server_address = address.to_string();
        let root_dir = root_dir.to_string_lossy().to_string();

        let handle = thread::spawn(move || {
            let server = Server::new(&server_address, &root_dir);
            let _ = server.run();
        });

        thread::sleep(Duration::from_millis(100));
        return handle;
    }

    #[test]
    fn test_server_responds_to_request() {
        let address = "127.0.0.1:8080";
        let temp_dir = tempdir().unwrap();
        let index_file = temp_dir.path().join("index.html");
        fs::write(&index_file, "Hello From Xener Server!").expect("Failed to write index file");

        let server_handle = start_test_server(address, temp_dir.path().to_path_buf());

        let mut stream = TcpStream::connect(address).expect("Failed to connect to test server");

        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        stream
            .write_all(request.as_bytes())
            .expect("Failed to send request");

        let mut buffer = Vec::new();
        loop {
            let mut temp = [0; 1024];
            let bytes_read = stream.read(&mut temp).expect("Failed to read response");
            if bytes_read == 0 {
                break;
            }
            buffer.extend_from_slice(&temp[..bytes_read]);
        }
        let response = String::from_utf8_lossy(&buffer);

        assert!(
            response.starts_with("HTTP/1.1 200 OK"),
            "Response should start with HTTP/1.1 200 OK"
        );
        assert!(
            response.contains("Hello From Xener Server!"),
            "Response should contain 'Hello From Xener Server!'"
        );
    }

    #[test]
    fn test_server_handles_invalid_request() {
        let address = "127.0.0.1:8081";
        let server_handle = start_test_server(address, tempdir().unwrap().path().to_path_buf());

        let mut stream = TcpStream::connect(address).expect("Failed to connect to test server");

        let request = "INVALID REQUEST\r\n\r\n";
        stream
            .write_all(request.as_bytes())
            .expect("Failed to send request");

        let mut buffer = [0; 1024];
        let bytes_read = stream.read(&mut buffer).expect("Failed to read response");

        assert!(bytes_read > 0, "Server should send some response");
    }
}
