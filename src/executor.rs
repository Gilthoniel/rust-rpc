
use std::io;
use std::thread::{spawn, JoinHandle};
use std::sync::{mpsc, Arc, Mutex};

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

type Job = Box<dyn FnBox + Send + 'static>;

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

  pub fn execute<F>(&self, f: F)
  where
    F: FnOnce() -> io::Result<()> + Send + 'static
  {
    let job = Box::new(f);

    self.sender.as_ref().unwrap().send(job).unwrap();
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    self.sender = None;

    let mut ids = Vec::new();

    for worker in self.workers.drain(..) {
      worker.thread.join().unwrap();
      ids.push(worker.id);
    }

    println!("Workers {:?} have been shutdown", ids);
  }
}

struct Worker {
  id: usize,
  thread: JoinHandle<()>,
}

impl Worker {
  fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
    let thread = spawn(move || {
      loop {
        if let Ok(job) = receiver.lock().unwrap().recv() {
          match job.call_box() {
            Err(e) => println!("Job {} failed: {}", id, e),
            Ok(_) => (),
          }
        } else {
          // Channel has been closed to shutdown.
          return;
        }
      }
    });

    Worker {
      id,
      thread,
    }
  }
}
