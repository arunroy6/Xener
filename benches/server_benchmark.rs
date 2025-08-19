use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

fn benchmark_concurrent_requests(c: &mut Criterion) {
    let num_clients = 100;
    let requests_per_client = 10;

    c.bench_function("concurrent_requests", |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(num_clients));

            let mut handles = Vec::with_capacity(num_clients);

            for _ in 0..num_clients {
                let barrier_clone = Arc::clone(&barrier);

                let handle = thread::spawn(move || {
                    barrier_clone.wait();

                    let start = Instant::now();
                    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(30)))
                        .unwrap();
                    stream
                        .set_write_timeout(Some(Duration::from_secs(30)))
                        .unwrap();

                    for i in 0..requests_per_client {
                        let request = if i == requests_per_client - 1 {
                            "GET / HTTP/1.1\r\nHost: localhost\r\n Connection: close\r\n\r\n"
                        } else {
                            "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n"
                        };

                        if stream.write_all(request.as_bytes()).is_err() {
                            break;
                        }

                        if read_http_response(&mut stream).is_err() {
                            break;
                        }
                    }

                    start.elapsed()
                });

                handles.push(handle);
            }

            let mut total_duration = Duration::new(0, 0);
            for handle in handles {
                total_duration += handle.join().unwrap();
            }

            black_box(total_duration / num_clients as u32)
        })
    });
}

fn read_http_response(stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(stream);

    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;

    let mut content_length = 0;
    let mut chunked = false;

    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;

        if header.trim().is_empty() {
            break; // End of headers
        }

        if header.to_lowercase().starts_with("content-length:") {
            content_length = header
                .split(':')
                .nth(1)
                .unwrap_or("0")
                .trim()
                .parse::<usize>()
                .unwrap_or(0);
        }

        if header.to_lowercase().starts_with("transfer-encoding:")
            && header.to_lowercase().contains("chunked")
        {
            chunked = true;
        }
    }

    if chunked {
        let mut buffer = vec![0u8; 1024];
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
        }
    } else if content_length > 0 {
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body)?;
    }

    Ok(())
}

criterion_group!(benches, benchmark_concurrent_requests);
criterion_main!(benches);
