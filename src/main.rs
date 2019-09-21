extern crate serde;

pub mod group;
pub mod transport;

use group::Address;
use std::fmt;
use std::io;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use transport::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum RPCError {
    BadRequest,
    NoMatchingResponse,
    DecodingError(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request<T> {
    Data(T),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response<T> {
    Error(RPCError),
    Data(T),
}

pub struct Server {
    stop_tx: Option<mpsc::SyncSender<()>>,
    addr: Option<Address>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            stop_tx: None,
            addr: None,
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

        std::thread::spawn(move || {
            let p = Arc::new(p);

            loop {
                match t.next(Arc::clone(&p)) {
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => match rx.try_recv() {
                        Ok(_) => return, // received close announcement
                        _ => t.wait(Duration::from_millis(100)).unwrap(),
                    },
                    Err(e) => {
                        println!("Error: {}", e);
                        return;
                    }
                    _ => (),
                }
            }
        });
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

        println!("{} has been closed.", self);
    }
}

#[cfg(test)]
mod tests {
    use super::group::Address;
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn hello_world() {
        #[rpc_macro::service]
        trait Hello {
            fn hello(&self, arg: String) -> String;
        }

        struct HelloService;

        impl Hello for HelloService {
            fn hello(&self, arg: String) -> String {
                arg
            }
        }

        let addr = Address::from_str("127.0.0.1:2000");

        let mut srv = Server::new();
        let service = HelloService {};
        srv.run(
            service.get_processor(),
            TcpServerTransport::new(addr.clone()).unwrap(),
        );

        let c = HelloClient::new(TcpClientTransport::new(addr));
        let msg = String::from("deadbeef");
        let r = c.hello(msg.clone()).unwrap();
        assert_eq!(msg, r);
    }

    #[test]
    fn counter() {
        #[rpc_macro::service]
        trait Counter {
            fn counter(&self, v: u64) -> u64;
            fn fetch(&self, v: u64) -> u64;
        }

        struct CounterService {
            value: AtomicU64,
        }

        impl Counter for CounterService {
            fn counter(&self, v: u64) -> u64 {
                let prev = self.value.fetch_add(v, Ordering::Relaxed);
                prev + v
            }

            fn fetch(&self, _: u64) -> u64 {
                self.value.load(Ordering::Relaxed)
            }
        }

        let addr = Address::from_str("127.0.0.1:2001");

        let mut srv = Server::new();
        let service = CounterService {
            value: AtomicU64::new(0),
        };
        srv.run(
            service.get_processor(),
            TcpServerTransport::new(addr.clone()).unwrap(),
        );

        let mut threads = Vec::new();
        let n = 5;
        let k = 10;
        for _ in 0..n {
            let addr = addr.clone();
            let h = std::thread::spawn(move || {
                let c = CounterClient::new(TcpClientTransport::new(addr));

                for _ in 0..k {
                    c.counter(1).unwrap();
                }
            });

            threads.push(h);
        }

        for th in threads {
            th.join().unwrap();
        }

        let c = CounterClient::new(TcpClientTransport::new(addr));
        let r = c.fetch(0).unwrap();
        assert_eq!(r, n * k);
    }
}
