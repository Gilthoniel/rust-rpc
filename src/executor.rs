use std::io;
use std::panic;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{Builder, JoinHandle};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

trait FnBox {
    fn call_box(self: Box<Self>) -> io::Result<()>;
}

impl<F: FnOnce() -> io::Result<()>> FnBox for F {
    fn call_box(self: Box<F>) -> io::Result<()> {
        (*self)()
    }
}

type Job = Box<dyn FnBox + Send + panic::UnwindSafe + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let mut workers = Vec::with_capacity(size);
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        for id in 0..size {
            workers.push(Worker::new(id, receiver.clone()));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F) -> io::Result<()>
    where
        F: FnOnce() -> io::Result<()> + Send + panic::UnwindSafe + 'static,
    {
        let job = Box::new(f);

        let sender = self.sender.as_ref().unwrap();

        if let Err(ref e) = sender.send(job) {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()));
        }

        Ok(())
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.sender = None;

        let mut ids = Vec::new();

        for worker in self.workers.drain(..) {
            match worker.thread.join() {
                Ok(_) => ids.push(worker.id),
                Err(e) => println!("Worker {} couldn't stop: {:?}", worker.id, e),
            };
        }

        println!("Workers {:?} have been shutdown.", ids);
    }
}

struct Worker {
    id: usize,
    thread: JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = Builder::new()
            .name(format!("worker-thread-{}", id))
            .spawn(move || {
                loop {
                    let receiver = receiver.lock().unwrap();

                    if let Ok(job) = receiver.recv() {
                        // Force to unlock so that any panics occuring in the handler
                        // won't poison the lock.
                        drop(receiver);

                        // Note: some panic might not be unwind
                        let res = panic::catch_unwind(|| job.call_box());

                        match res {
                            Err(_) => {
                                println!("Worker {} caught a panic.", id);
                            },
                            Ok(res) => match res {
                                Err(e) => println!("Worker {} failed a job: {}.", id, e),
                                Ok(_) => (),
                            }
                        }
                    } else {
                        // Channel has been closed to shutdown.
                        return;
                    }
                }
            })
            .unwrap();

        Worker { id, thread }
    }
}
