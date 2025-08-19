#[cfg(test)]
mod tests {
    use super::super::*;
    use reqwest::blocking::Client;
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::sync::Barrier;
    use std::time::Duration;
    use std::{fs, thread};
    use tempfile::tempdir;

    fn start_test_server(ip: &str, port: u16, root_dir: PathBuf) -> thread::JoinHandle<()> {
        let root_dir = root_dir.to_string_lossy().to_string();
        let server_config = Arc::new(ServerConfig::with_params(ip, port, 1, &root_dir));

        let handle = thread::spawn(move || {
            let server = Server::new(server_config);
            let _ = server.run();
        });

        thread::sleep(Duration::from_millis(100));
        return handle;
    }

    #[test]
    fn test_server_responds_to_request() {
        let temp_dir = tempdir().unwrap();
        let index_file = temp_dir.path().join("index.html");
        fs::write(&index_file, "Hello From Xener Server!").expect("Failed to write index file");

        let _ = start_test_server("127.0.0.1", 8080, temp_dir.path().to_path_buf());

        let mut stream =
            TcpStream::connect("127.0.0.1:8080").expect("Failed to connect to test server");

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
        let _ = start_test_server("127.0.0.1", 8081, tempdir().unwrap().path().to_path_buf());

        let mut stream = TcpStream::connect(address).expect("Failed to connect to test server");

        let request = "INVALID REQUEST\r\n\r\n";
        stream
            .write_all(request.as_bytes())
            .expect("Failed to send request");

        let mut buffer = [0; 1024];
        let bytes_read = stream.read(&mut buffer).expect("Failed to read response");

        assert!(bytes_read > 0, "Server should send some response");
    }

    #[test]
    fn test_concurrent_connections() {
        let _ = start_test_server("127.0.0.1", 8082, tempdir().unwrap().path().to_path_buf());

        let client_count = 5;
        let barrier = Arc::new(Barrier::new(client_count));
        let mut handles = Vec::with_capacity(client_count);

        for i in 0..client_count {
            let barrier_clone = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                barrier_clone.wait();
                let mut stream = TcpStream::connect("127.0.0.1:8082").unwrap();

                let request = format!("GET /client{} HTTP/1.1\r\nHost: localhost\r\n\r\n", i);
                stream.write_all(request.as_bytes()).unwrap();

                let mut buffer = [0; 1024];
                let bytes_read = stream.read(&mut buffer).unwrap();
                let response = String::from_utf8_lossy(&buffer[0..bytes_read]);

                assert!(
                    response.contains("HTTP/1.1"),
                    "Server should send some response"
                );
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_keep_alive_connection() {
        let temp_dir = tempdir().unwrap();
        let index_file = temp_dir.path().join("index.html");
        fs::write(&index_file, "Hello From Xener Server!").expect("Failed to write index file");

        let _ = start_test_server("127.0.0.1", 8083, temp_dir.path().to_path_buf());

        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        for i in 0..5 {
            let response = client
                .get(format!("http://{}/index.html", "127.0.0.1:8083"))
                .header("Connection", "keep-alive")
                .send()
                .expect("failed to send request");

            assert!(
                response.status().is_success(),
                "Request {} failed with status {}",
                i,
                response.status()
            );

            let headers = response.headers();
            assert!(
                headers.contains_key("connection"),
                "Response missing Connection header"
            );
        }
    }
}
