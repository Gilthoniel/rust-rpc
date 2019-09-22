extern crate mio;
extern crate serde;

use super::super::{
    executor::ThreadPool,
    group::Address,
    RequestProcessor,
    Response,
    RpcError,
    RpcResult,
    ServerError,
};
use super::*;
use mio::net::TcpListener;
use mio::{Events, Poll, PollOpt, Ready, Token};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Debug;
use std::{io, io::{Read, Write}};
use std::net::{Shutdown, TcpStream};

const READ_TIMEOUT: Option<Duration> = Some(Duration::from_millis(5000));
const WRITE_TIMEOUT: Option<Duration> = Some(Duration::from_millis(5000));

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

impl<Req: Debug + 'static, Rep: Debug + 'static> ServerTransport<Req, Rep> for TcpServerTransport
where
    for<'de> Req: Deserialize<'de>,
    Rep: Serialize,
{
    /// Get the socket address of the server.
    fn get_addr(&self) -> Address {
        self.addr.clone()
    }

    /// Try to bind to the socket address and set the socket if
    /// successfull, otherwise the result contains the error.
    fn connect(&mut self) -> RpcResult<()> {
        let socket_addr = match self.addr.get_socket_addr() {
            Some(addr) => addr,
            None => return Err(RpcError::NoSocketAddress),
        };
        let socket = TcpListener::bind(&socket_addr)?;

        self.poll
            .register(&socket, Token(0), Ready::readable(), PollOpt::edge())?;

        self.socket = Some(socket);

        Ok(())
    }

    /// Wait for a connection request and read incoming data. It will
    /// then process the message and write the reply to the stream.
    fn next(&self, f: Arc<Box<RequestProcessor<Req, Rep>>>) -> RpcResult<()> {
        let socket = match self.socket.as_ref() {
            Some(socket) => socket,
            None => return Err(RpcError::NotRunning),
        };

        let (mut stream, _) = socket.accept_std()?;

        self.pool.execute(move || -> io::Result<()> {
            stream.set_read_timeout(READ_TIMEOUT)?;
            stream.set_write_timeout(WRITE_TIMEOUT)?;

            let mut buf = Vec::new();
            stream.read_to_end(&mut buf)?;

            let out: Vec<u8>;
            match serde_json::from_slice(&buf[..]) {
                Ok(msg) => {
                    let reply = f(msg);

                    out = serde_json::to_vec(&reply)?;
                }
                Err(e) => {
                    let desc = String::from(e.description());
                    let reply: Response<Rep> = Response::Error(ServerError::DecodingError(desc));

                    out = serde_json::to_vec(&reply)?;
                }
            };

            stream.write_all(&out[..])?;
            Ok(())
        })?;

        Ok(())
    }

    fn wait(&mut self, timeout: Duration) -> RpcResult<()> {
        self.poll.poll(&mut self.events, Some(timeout))?;

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
    /// Create a connection object that can be used to connect to a
    /// server and send messages.
    fn connect(&self) -> Connection<Req, Rep> {
        let addr = self.addr.clone();

        Connection {
            tx: Box::new(move |msg| {
                let socket_addr = match addr.get_socket_addr() {
                    Some(addr) => addr,
                    None => return Err(RpcError::NoSocketAddress),
                };

                let mut stream = TcpStream::connect(socket_addr)?;

                let bin = serde_json::to_vec(&msg)?;

                stream.write_all(&bin)?;
                stream.shutdown(Shutdown::Write)?;

                let mut buf = Vec::new();
                stream.read_to_end(&mut buf)?;

                Ok(serde_json::from_reader(&buf[..])?)
            }),
        }
    }
}
