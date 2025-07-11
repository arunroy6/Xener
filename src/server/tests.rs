#[cfg(test)]
mod tests {
    use std::thread;
    use std::{time::Duration};
    use super::super::*;

    fn start_test_server(address: &str) -> thread::JoinHandle<()> {
        let server_address = address.to_string();

        thread::spawn(move || {
            let server = Server::new(&server_address);
            let _ = server.run();
        })
    }

    #[test]
    fn test_server_responds_to_request() {
        let address = "127.0.0.1:8080";
        let _ = start_test_server(address);

        thread::sleep(Duration::from_millis(100));

        let mut stream = TcpStream::connect(address)
            .expect("Failed to connect to test server");

        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        stream.write_all(request.as_bytes())
            .expect("Failed to send request");

        let mut buffer = [0; 1024];
        let bytest_read = stream.read(&mut buffer)
            .expect("Failed to read response");

        let response = String::from_utf8_lossy(&buffer[0..bytest_read]);

        assert!(response.starts_with("HTTP/1.1 200 OK"), "Response should start with HTTP/1.1 200 OK");
        assert!(response.contains("Hello, World!"), "Response should contain 'Hello, World!'");
    }
    
    #[test]
    fn test_server_handles_invalid_request() {
        let address = "127.0.0.1:8080";
        let _ = start_test_server(address);
        thread::sleep(Duration::from_millis(100));

        let mut stream = TcpStream::connect(address)
            .expect("Failed to connect to test server");

        let request = "INVALID REQUEST\r\n\r\n";
        stream.write_all(request.as_bytes())
            .expect("Failed to send request");

        let mut buffer = [0; 1024];
        let bytes_read = stream.read(&mut buffer)
            .expect("Failed to read response");

        assert!(bytes_read > 0, "Server should send some response");
    }

}