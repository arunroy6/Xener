use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use tracing::{debug, error, info};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    sender: Option<mpsc::Sender<Job>>,
    workers: Vec<Worker>,
    size: usize,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let size = if size > 0 { size } else { 1 };
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        info!("Creating thread pool with {} workers", size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            sender: Some(sender),
            workers,
            size,
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        if let Some(sender) = &self.sender {
            if let Err(e) = sender.send(job) {
                error!("Error sending job to thread pool: {:?}", e);
            }
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for ThreadPool {
    // Drop the sender to signal workers to terminate
    // This is important because dropping the sender will cause
    // all receivers to receive an error when they try to recv()
    fn drop(&mut self) {
        drop(self.sender.take());

        info!("Shutting down thread pool, waiting for workers to finish");

        // Wait for all workers to finish
        for worker in &mut self.workers {
            debug!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                if let Err(e) = thread.join() {
                    error!("Error joining worker thread {}: {:?}", worker.id, e);
                }
            }
        }
        info!("Thread pool shutdown completed");
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::Builder::new()
            .name(format!("Worker-{}", id))
            .spawn(move || {
                debug!("Worker {} started", id);

                loop {
                    let message = {
                        let receiver = match receiver.lock() {
                            Ok(lock) => lock,
                            Err(poisoned) => {
                                error!("Worker {} encountered a poised mutex", id);

                                poisoned.into_inner()
                            }
                        };
                        // This will block until a job available or the sender is dropped
                        receiver.recv()
                    };

                    match message {
                        Ok(job) => {
                            debug!("Worker {} got a job: executing", id);
                            job();
                        }
                        Err(_) => {
                            debug!("Worker {} shutting down", id);
                            break;
                        }
                    }
                }
            })
            .expect("Failed to spawn thread");
        Worker {
            id,
            thread: Some(thread),
        }
    }
}
