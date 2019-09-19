extern crate serde;

use super::super::{group::Address, RequestProcessor};
use super::*;
use serde::{Deserialize, Serialize};
use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

/// ServerTransport implementation over TCP and using
/// JSON to serialize the messages.
pub struct TcpServerTransport {
    addr: Address,
    socket: Option<TcpListener>,
}

impl TcpServerTransport {
    /// Create a transport object. The socket will be bind to
    /// the given address.
    pub fn new(addr: Address) -> TcpServerTransport {
        TcpServerTransport { addr, socket: None }
    }
}

impl<Req: 'static, Rep: 'static> ServerTransport<Req, Rep> for TcpServerTransport
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
    fn connect(&mut self) -> io::Result<()> {
        let socket = TcpListener::bind(self.addr.clone())?;
        socket.set_nonblocking(true)?;

        self.socket = Some(socket);

        Ok(())
    }

    /// Wait for a connection request and read incoming data. It will
    /// then process the message and write the reply to the stream.
    fn next(&self, f: Arc<Box<RequestProcessor<Req, Rep>>>) -> io::Result<()> {
        let socket = self.socket.as_ref().unwrap();

        let (mut stream, _) = socket.accept()?;

        thread::spawn(move || -> io::Result<()> {
            let mut buf = [0; 128];
            let size = stream.read(&mut buf)?;

            let msg = serde_json::from_slice(&buf[0..size])?;

            let reply = f(msg);

            let bout = serde_json::to_vec(&reply)?;
            stream.write_all(&bout[..])?;
            Ok(())
        });

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
