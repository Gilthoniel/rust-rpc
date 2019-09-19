use std::fmt;
use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::option::IntoIter;
use std::str::FromStr;

/// Address can be local or distant. A local address will have
/// a unique identifier and a distant address will have an ip
/// and a port.
#[derive(Clone, Debug)]
pub enum Address {
    Local(String),
    Socket(SocketAddr),
}

impl Address {
    pub fn get_socket_addr(&self) -> Option<SocketAddr> {
        match self {
            Address::Socket(addr) => Some(addr.clone()),
            _ => None,
        }
    }

    /// Create an address from an IP and a port. The resulting
    /// address will be distant.
    pub fn from_ip(ip: IpAddr, port: u16) -> Address {
        Address::Socket(SocketAddr::new(ip, port))
    }

    /// Create an address from a string. If it can be parsed
    /// into a socket address, it will create a distant address
    /// and a local one for any other case.
    pub fn from_str(addr: &str) -> Address {
        let r = SocketAddr::from_str(addr);

        match r {
            Ok(addr) => Address::Socket(addr),
            Err(_) => Address::Local(String::from(addr)),
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Local(value) => write!(f, "{}", value),
            Address::Socket(addr) => write!(f, "{}", addr),
        }
    }
}

impl ToSocketAddrs for Address {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<IntoIter<SocketAddr>> {
        match self {
            Address::Socket(addr) => Ok(Some(addr.clone()).into_iter()),
            _ => Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "not a socket type address",
            )),
        }
    }
}
