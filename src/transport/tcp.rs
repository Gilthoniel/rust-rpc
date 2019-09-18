extern crate serde;

use std::io::{Read, Write};
use std::net::TcpStream;
use serde::{Deserialize, Serialize};
use super::*;
use super::super::{RequestProcessor, group::Address};

pub struct TcpServerTransport {
  addr: Address,
}

impl TcpServerTransport {
  pub fn new(addr: Address) -> TcpServerTransport {
    TcpServerTransport{ addr }
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

pub struct TcpClientTransport {
  addr: Address,
}

impl TcpClientTransport {
  pub fn new(addr: Address) -> TcpClientTransport {
    TcpClientTransport { addr }
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
