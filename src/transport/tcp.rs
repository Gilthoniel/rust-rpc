extern crate serde;

use std::io;
use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener};
use serde::{Deserialize, Serialize};
use super::*;
use super::super::{RequestProcessor, group::Address};

pub struct TcpServerTransport {
  addr: Address,
  socket: Option<TcpListener>,
}

impl TcpServerTransport {
  pub fn new(addr: Address) -> TcpServerTransport {
    TcpServerTransport{
      addr,
      socket: None,
    }
  }
}

impl<Req, Rep> ServerTransport<Req, Rep> for TcpServerTransport
where
  for<'de> Req: Deserialize<'de>,
  Rep: Serialize
{
  fn get_addr(&self) -> Address {
    self.addr.clone()
  }

  fn connect(&mut self) -> io::Result<()> {
    let socket = TcpListener::bind(self.addr.clone())?;
    socket.set_nonblocking(true)?;

    self.socket = Some(socket);

    Ok(())
  }

  fn next(&self, f: &RequestProcessor<Req, Rep>) -> io::Result<()> {
    let socket = self.socket.as_ref().unwrap();

    let (mut stream, _) = socket.accept()?;

    let mut buf = [0; 128];
    let size = stream.read(&mut buf)?;

    let msg = serde_json::from_slice(&buf[0..size])?;

    let reply = f(msg);

    let bout = serde_json::to_vec(&reply)?;
    stream.write_all(&bout[..])?;

    Ok(())
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
