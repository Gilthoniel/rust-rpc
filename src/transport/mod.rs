mod tcp;

use std::io;
use super::group::Address;

pub use tcp::*;

pub type RequestProcessor<Req, Rep> = dyn Fn(Req) -> Rep + Send + Sync;

pub trait ServerTransport<Req, Rep>: Send + Sync + 'static {
  fn get_addr(&self) -> Address;
  fn connect(&mut self) -> io::Result<()>;
  fn next(&self, f: &RequestProcessor<Req, Rep>) -> io::Result<()>;
}

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
