extern crate serde;

mod error;
pub mod executor;
pub mod group;
pub mod transport;

pub use error::*;

use group::Address;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;
use transport::*;

pub type ServiceResult<T> = Result<T, String>;

#[derive(Serialize, Deserialize, Debug)]
pub enum Request<T> {
    Data(T),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response<T> {
    Error(ServerError),
    Data(T),
}

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
                match t.next(Arc::clone(&p)) {
                    Err(ref e) if e.would_block() => match rx.try_recv() {
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

#[cfg(test)]
mod tests {
    use super::group::Address;
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn hello_world() -> RpcResult<()> {
        #[rpc_macro::service]
        trait Hello {
            fn hello(&self, arg: String) -> ServiceResult<String>;
        }

        struct HelloService;

        impl Hello for HelloService {
            fn hello(&self, arg: String) -> ServiceResult<String> {
                Ok(arg)
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
        let r = c.hello(msg.clone())?;
        assert_eq!(msg, r);

        Ok(())
    }

    const COUNTER_ERROR: &str = "increment must be above zero";

    #[test]
    #[should_panic(expected = "increment must be above zero")]
    fn counter() {
        #[rpc_macro::service]
        trait Counter {
            fn counter(&self, v: u64) -> ServiceResult<u64>;
            fn fetch(&self, v: u64) -> ServiceResult<u64>;
        }

        struct CounterService {
            value: AtomicU64,
        }

        impl Counter for CounterService {
            fn counter(&self, v: u64) -> ServiceResult<u64> {
                if v == 0 {
                    return Err(COUNTER_ERROR.to_string());
                }

                let prev = self.value.fetch_add(v, Ordering::Relaxed);
                Ok(prev + v)
            }

            fn fetch(&self, _: u64) -> ServiceResult<u64> {
                Ok(self.value.load(Ordering::Relaxed))
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
            let h = std::thread::spawn(move || -> RpcResult<()> {
                let c = CounterClient::new(TcpClientTransport::new(addr));

                for _ in 0..k {
                    c.counter(1)?;
                }

                Ok(())
            });

            threads.push(h);
        }

        for th in threads {
            th.join().unwrap().unwrap();
        }

        let c = CounterClient::new(TcpClientTransport::new(addr));
        let r = c.fetch(0).unwrap();
        assert_eq!(r, n * k);

        // This is where it should panic.
        c.counter(0).unwrap();
    }

    #[test]
    fn error() -> RpcResult<()> {
        #[rpc_macro::service]
        trait Byzantine {
            fn byzantine(&self, arg: u64) -> ServiceResult<u64>;
        }

        struct ByzantineService;

        impl Byzantine for ByzantineService {
            fn byzantine(&self, _: u64) -> ServiceResult<u64> {
                panic!("example panic in test");
            }
        }

        let addr = Address::from_str("127.0.0.1:2002");

        let mut srv = Server::new();
        let service = ByzantineService;

        srv.run(
            service.get_processor(),
            TcpServerTransport::new(addr.clone()).unwrap(),
        );

        let c = ByzantineClient::new(TcpClientTransport::new(addr));
        assert!(c.byzantine(0).is_err());

        Ok(())
    }
}
