mod tcp;

use super::{group::Address, Request, Response, RpcResult};
use std::sync::Arc;
use std::time::Duration;

pub use tcp::*;

/// Processor created by services that will be used by the server
/// to process the requests sent by the clients.
pub type RequestProcessor<Req, Rep> = dyn Fn(Request<Req>) -> Response<Rep> + Send + Sync;

/// A server transport defines how the server will receive requests.
pub trait ServerTransport<Req, Rep>: Send + 'static {
    /// Get the address that identify the server.
    fn get_addr(&self) -> Address;
    /// Start to listen for incoming requests.
    fn connect(&mut self) -> RpcResult<()>;
    /// Serve the next incoming connection using the processor
    /// to generate the response.
    fn next(&self, f: Arc<Box<RequestProcessor<Req, Rep>>>) -> RpcResult<()>;
    /// Sleep until a IO event is triggered on the transport.
    fn wait(&mut self, timeout: Duration) -> RpcResult<()>;
}

/// Connection created by a client transport to talk with a server.
pub struct Connection<Req, Rep> {
    tx: Box<dyn Fn(Req) -> RpcResult<Rep>>,
}

impl<Req, Rep> Connection<Req, Rep> {
    /// Send a message to the server and returns the reply.
    pub fn send(&self, msg: Req) -> RpcResult<Rep> {
        (self.tx)(msg)
    }
}

// A client transport defines how the client will talk to the server.
pub trait ClientTransport<Req, Rep> {
    /// Create a connection that can be used to send messages to
    /// a server.
    fn connect(&self) -> Connection<Req, Rep>;
}
