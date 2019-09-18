extern crate serde;

pub mod group;
pub mod transport;

use transport::*;

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

#[cfg(test)]
mod tests {
  use super::*;
  use super::group::Address;

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

    let srv = Server::new();
    let service = HelloService{};
    srv.run(Box::new(service.get_processor()), TcpServerTransport::new(addr.clone()));

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

    let addr = Address::from_str("127.0.0.1:2001");

    let srv = Server::new();
    let service = CounterService{value: 0};
    srv.run(Box::new(service.get_processor()), TcpServerTransport::new(addr.clone()));

    let c = CounterClient::new(TcpClientTransport::new(addr));
    c.counter(1).unwrap();
    let r = c.counter(2).unwrap();
    assert_eq!(r, 3);
  }
}
