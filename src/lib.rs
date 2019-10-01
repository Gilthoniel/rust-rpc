extern crate serde;

pub mod executor;
pub mod group;
pub mod transport;

pub use rpc_macro::service;

use group::Address;
use transport::{
    RequestProcessor,
    ServerTransport,
};
use std::fmt;
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

pub struct Server {
    stop_tx: Option<mpsc::SyncSender<()>>,
    addr: Option<Address>,
    th: Option<JoinHandle<()>>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            stop_tx: None,
            addr: None,
            th: None,
        }
    }

    pub fn run<Req: 'static, Rep: 'static>(
        &mut self,
        p: Box<RequestProcessor<Req, Rep>>,
        t: impl ServerTransport<Req, Rep>,
    ) {
        self.addr = Some(t.get_addr());
        let mut t = t;

        let (tx, rx) = mpsc::sync_channel(1);
        self.stop_tx = Some(tx);

        t.connect().unwrap();

        println!("{} has started. Listening for incoming requests...", self);

        self.th = Some(std::thread::spawn(move || {
            let p = Arc::new(p);

            loop {
                match rx.try_recv() {
                    Ok(_) => return, // received close announcement
                    _ => (),
                };

                if let Err(e) = t.next(Arc::clone(&p)) {
                    println!("Error: {:?}", e);
                }
            }
        }));
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Server[{}]", self.addr.as_ref().unwrap())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let tx = self.stop_tx.as_ref().unwrap();

        if let Err(ref e) = tx.send(()) {
            println!("Close error: {}", e);
        }

        // Make sure the main thread is stopped.
        self.th.take().unwrap().join().unwrap();

        println!("{} has been closed.", self);
    }
}
