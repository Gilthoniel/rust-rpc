extern crate mio;
extern crate serde;

use super::super::{
    executor::ThreadPool,
    group::Address,
    RequestProcessor,
};
use super::{ServerTransport, ClientTransport};
use mio::net::TcpListener;
use mio::{Events, Poll, PollOpt, Ready, Token};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::{io, io::{Read, Write}};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;
use std::sync::Arc;
use std::error;

const WAIT_TIMEOUT: Option<Duration> = Some(Duration::from_millis(100));
const READ_TIMEOUT: Option<Duration> = Some(Duration::from_millis(5000));
const WRITE_TIMEOUT: Option<Duration> = Some(Duration::from_millis(5000));

#[derive(Serialize, Deserialize, Debug)]
pub enum Error {
    IoError(String),
    SerdeError(String),
    NoSocketAddress,
    NotRunning,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(error::Error::description(&err).to_string())
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        Error::SerdeError(error::Error::description(&err).to_string())
    }
}

/// ServerTransport implementation over TCP and using
/// JSON to serialize the messages.
pub struct TcpServerTransport {
    addr: Address,
    socket: Option<TcpListener>,
    pool: ThreadPool,
    poll: Poll,
    events: Events,
}

impl TcpServerTransport {
    /// Create a transport object. The socket will be bind to
    /// the given address.
    pub fn new(addr: Address) -> io::Result<TcpServerTransport> {
        let poll = Poll::new()?;

        Ok(TcpServerTransport {
            addr,
            socket: None,
            pool: ThreadPool::new(4),
            poll,
            events: Events::with_capacity(1),
        })
    }
}

impl<Req, Rep> ServerTransport<Req, Rep> for TcpServerTransport
where
    for<'de> Req: Debug + Deserialize<'de> + 'static,
    Rep: Debug + Serialize + 'static,
{
    type Error = Error;

    /// Get the socket address of the server.
    fn get_addr(&self) -> Address {
        self.addr.clone()
    }

    /// Try to bind to the socket address and set the socket if
    /// successfull, otherwise the result contains the error.
    fn connect(&mut self) -> Result<(), Error> {
        let socket_addr = match self.addr.get_socket_addr() {
            Some(addr) => addr,
            None => return Err(Error::NoSocketAddress),
        };
        let socket = TcpListener::bind(&socket_addr)?;

        self.poll
            .register(&socket, Token(0), Ready::readable(), PollOpt::edge())?;

        self.socket = Some(socket);

        Ok(())
    }

    /// Wait for a connection request and read incoming data. It will
    /// then process the message and write the reply to the stream.
    fn next(&mut self, f: Arc<Box<RequestProcessor<Req, Rep>>>) -> Result<(), Error> {
        let socket = match self.socket.as_ref() {
            Some(socket) => socket,
            None => return Err(Error::NotRunning),
        };

        let (mut stream, _) = match socket.accept_std() {
            Ok(v) => v,
            Err(e) => {
                // A WouldBlock error only means no connection yet
                // so we need to wait a bit.
                if e.kind() == io::ErrorKind::WouldBlock {
                    // Wait for an IO event from the OS.
                    self.poll.poll(&mut self.events, WAIT_TIMEOUT)?;
                    // As the next is looped, we simply return to try
                    // a new accept.
                    return Ok(());
                }

                return Err(Error::from(e));
            }
        };

        self.pool.execute(move || -> io::Result<()> {
            stream.set_read_timeout(READ_TIMEOUT)?;
            stream.set_write_timeout(WRITE_TIMEOUT)?;

            let mut buf = Vec::new();
            stream.read_to_end(&mut buf)?;

            let reply = f(serde_json::from_slice(&buf[..])?);
            let out = serde_json::to_vec(&reply)?;

            stream.write_all(&out[..])?;
            Ok(())
        })?;

        Ok(())
    }
}

/// ClientTransport implementation over TCP.
pub struct TcpClientTransport {
    addr: Address,
}

impl TcpClientTransport {
    /// Create a client transport that will try to connect
    /// to the server at the given address.
    pub fn new(addr: Address) -> TcpClientTransport {
        TcpClientTransport { addr }
    }
}

impl<Req, Rep> ClientTransport<Req, Rep> for TcpClientTransport
where
    for<'de> Rep: Deserialize<'de>,
    Req: Serialize,
{
    type Error = Error;

    /// Create a connection object that can be used to connect to a
    /// server and send messages.
    fn send(&self, msg: Req) -> Result<Rep, Error> {
        let socket_addr = match self.addr.get_socket_addr() {
            Some(addr) => addr,
            None => return Err(Error::NoSocketAddress),
        };

        let mut stream = TcpStream::connect(socket_addr)?;

        let bin = serde_json::to_vec(&msg)?;

        stream.write_all(&bin)?;
        stream.shutdown(Shutdown::Write)?;

        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;

        Ok(serde_json::from_reader(&buf[..])?)
    }
}
