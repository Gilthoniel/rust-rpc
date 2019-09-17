extern crate serde;
extern crate serde_json;

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;

pub type RequestProcessor<Req, Rep> = dyn FnMut(Req) -> Rep + Send + Sync;

pub trait ServerTransport<Req, Rep>: Send + Sync + 'static {
  fn listen(&self, f: Box<RequestProcessor<Req, Rep>>);
}

pub struct TcpServerTransport {
  addr: String,
}

impl TcpServerTransport {
  pub fn new(addr: &str) -> TcpServerTransport {
    TcpServerTransport{ addr: String::from(addr) }
  }
}

impl<Req, Rep> ServerTransport<Req, Rep> for TcpServerTransport
where
  for<'de> Req: Deserialize<'de>,
  Rep: Serialize
{
  fn listen(&self, f: Box<RequestProcessor<Req, Rep>>) {
    let listener = std::net::TcpListener::bind(self.addr.clone()).unwrap();

    let mut f = f;

    for stream in listener.incoming() {
      let mut stream = stream.unwrap();

      let mut buf = [0; 128];
      let size = stream.read(&mut buf).unwrap();

      let msg = serde_json::from_slice(&buf[0..size]).unwrap();

      let reply = f(msg);

      let bout = serde_json::to_vec(&reply).unwrap();
      stream.write_all(&bout[..]).unwrap();
    }
  }
}

pub struct Server;

impl Server {
  pub fn new() -> Server {
    Server {}
  }

  pub fn run<Req: 'static, Rep: 'static>(&self, p: Box<RequestProcessor<Req, Rep>>, t: impl ServerTransport<Req, Rep>) {
    let p = Box::new(p);

    std::thread::spawn(move || {
      println!("Server has started. Listening for incoming requests...");

      t.listen(p);
    });
  }
}

// Client side -----------------------

pub struct Connection<Req, Rep> {
  tx: Box<dyn Fn(Req) -> Rep>,
}

impl<Req, Rep> Connection<Req, Rep> {
  pub fn send(&self, msg: Req) -> Rep {
    (self.tx)(msg)
  }
}

pub trait ClientTransport<Req, Rep> {
  fn connect(&self) -> Connection<Req, Rep>;
}

pub struct TcpClientTransport {
  addr: String,
}

impl TcpClientTransport {
  pub fn new(addr: &str) -> TcpClientTransport {
    TcpClientTransport { addr: String::from(addr) }
  }
}

impl<Req, Rep> ClientTransport<Req, Rep> for TcpClientTransport
where
  for<'de> Rep: Deserialize<'de>,
  Req: Serialize,
{
  fn connect(&self) -> Connection<Req, Rep> {
    let addr = self.addr.clone();

    Connection {
      tx: Box::new(move |msg| {
        let mut stream = TcpStream::connect(addr.clone()).unwrap();

        let bin = serde_json::to_vec(&msg).unwrap();
        stream.write_all(&bin).unwrap();

        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();

        serde_json::from_reader(&buf[..]).unwrap()
      }),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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

    let addr = "127.0.0.1:2000";

    let srv = Server::new();
    let service = HelloService{};
    srv.run(Box::new(service.get_processor()), TcpServerTransport::new(addr));

    let c = HelloClient::new(TcpClientTransport::new(addr));
    let msg = String::from("deadbeef");
    let r = c.hello(msg.clone()).unwrap();
    assert_eq!(msg, r);
  }

  #[test]
  fn counter() {
    #[rpc_macro::service]
    trait Counter {
      fn counter(&mut self, v: u64) -> u64;
    }

    struct CounterService {
      value: u64,
    }

    impl Counter for CounterService {
      fn counter(&mut self, v: u64) -> u64 {
        self.value += v;
        self.value
      }
    }

    let addr = "127.0.0.1:2001";

    let srv = Server::new();
    let service = CounterService{value: 0};
    srv.run(Box::new(service.get_processor()), TcpServerTransport::new(addr));

    let c = CounterClient::new(TcpClientTransport::new(addr));
    c.counter(1).unwrap();
    let r = c.counter(2).unwrap();
    assert_eq!(r, 3);
  }
}
