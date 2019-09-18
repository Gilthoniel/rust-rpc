mod tcp;

pub use tcp::*;

pub type RequestProcessor<Req, Rep> = dyn FnMut(Req) -> Rep + Send + Sync;

pub trait ServerTransport<Req, Rep>: Send + Sync + 'static {
  fn listen(&self, f: Box<RequestProcessor<Req, Rep>>);
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
