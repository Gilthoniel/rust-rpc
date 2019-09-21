mod tcp;

use super::{group::Address, Request, Response};
use std::io;
use std::sync::Arc;
use std::time::Duration;

pub use tcp::*;

/// Processor created by services that will be used by the server
/// to process the requests sent by the clients.
pub type RequestProcessor<Req, Rep> = dyn Fn(Request<Req>) -> Response<Rep> + Send + Sync;

/// A server transport defines how the server will receive requests.
pub trait ServerTransport<Req, Rep>: Send + Sync + 'static {
    /// Get the address that identify the server.
    fn get_addr(&self) -> Address;
    /// Start to listen for incoming requests.
    fn connect(&mut self) -> io::Result<()>;
    /// Serve the next incoming connection using the processor
    /// to generate the response.
    fn next(&self, f: Arc<Box<RequestProcessor<Req, Rep>>>) -> io::Result<()>;
    /// Sleep until a IO event is triggered on the transport.
    fn wait(&mut self, timeout: Duration) -> io::Result<()>;
}

/// Connection created by a client transport to talk with a server.
pub struct Connection<Req, Rep> {
    tx: Box<dyn Fn(Req) -> Rep>,
}

impl<Req, Rep> Connection<Req, Rep> {
    /// Send a message to the server and returns the reply.
    pub fn send(&self, msg: Req) -> Rep {
        (self.tx)(msg)
    }
}

// A client transport defines how the client will talk to the server.
pub trait ClientTransport<Req, Rep> {
    fn connect(&self) -> Connection<Req, Rep>;
}
