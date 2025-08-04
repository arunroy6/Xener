use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::io::{Read, Write};
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

                    for _ in 0..requests_per_client {
                        let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();

                        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
                        stream.write_all(request.as_bytes()).unwrap();

                        let mut buffer = [0; 1024];
                        let _ = stream.read(&mut buffer).unwrap();
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

criterion_group!(benches, benchmark_concurrent_requests);
criterion_main!(benches);
