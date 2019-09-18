use std::str::FromStr;
use std::option::IntoIter;
use std::net::{SocketAddr, IpAddr, ToSocketAddrs};
use std::io;

#[derive(Clone, Debug)]
pub enum Address {
  Local(String),
  Socket(SocketAddr),
}

impl Address {
  pub fn from_ip(ip: IpAddr, port: u16) -> Address {
    Address::Socket(SocketAddr::new(ip, port))
  }

  pub fn from_str(addr: &str) -> Address {
    let r = SocketAddr::from_str(addr);

    match r {
      Ok(addr) => Address::Socket(addr),
      Err(_) => Address::Local(String::from(addr)),
    }
  }
}

impl ToSocketAddrs for Address {
  type Iter = IntoIter<SocketAddr>;

  fn to_socket_addrs(&self) -> io::Result<IntoIter<SocketAddr>> {
    match self {
      Address::Socket(addr) => Ok(Some(addr.clone()).into_iter()),
      _ => Err(io::Error::new(io::ErrorKind::AddrNotAvailable, "not a socket type address")),
    }
  }
}
