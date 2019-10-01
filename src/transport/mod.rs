pub mod tcp;

use super::group::Address;
use std::sync::Arc;
use std::panic::RefUnwindSafe;

/// Processor created by services that will be used by the server
/// to process the requests sent by the clients.
pub type RequestProcessor<Req, Rep> = dyn Fn(Req) -> Rep + Send + Sync + RefUnwindSafe;

/// A server transport defines how the server will receive requests.
pub trait ServerTransport<Req, Rep>: Send + 'static {
    type Error: std::fmt::Debug;

    /// Get the address that identify the server.
    fn get_addr(&self) -> Address;
    /// Start to listen for incoming requests.
    fn connect(&mut self) -> Result<(), Self::Error>;
    /// Serve the next incoming connection using the processor
    /// to generate the response.
    fn next(&mut self, f: Arc<Box<RequestProcessor<Req, Rep>>>) -> Result<(), Self::Error>;
}

// A client transport defines how the client will talk to the server.
pub trait ClientTransport<Req, Rep> {
    type Error: std::error::Error;

    /// Create a connection that can be used to send messages to
    /// a server.
    fn send(&self, msg: Req) -> Result<Rep, Self::Error>;
}
