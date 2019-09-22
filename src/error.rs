use serde::{Deserialize, Serialize};
use std::io;
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerError {
    BadRequest,
    DecodingError(String),
    ProcessorError(String),
}

#[derive(Debug)]
pub enum RpcError {
    IoError(io::Error),
    SerdeError(serde_json::error::Error),
    ServerError(ServerError),
    InvalidResponseType,
    NoSocketAddress,
    NotRunning,
}

impl RpcError {
    pub fn from_server_err(err: ServerError) -> RpcError {
        RpcError::ServerError(err)
    }

    pub fn would_block(&self) -> bool {
      match self {
        RpcError::IoError(err) => err.kind() == io::ErrorKind::WouldBlock,
        _ => false,
      }
    }
}

impl fmt::Display for RpcError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      RpcError::IoError(err) => write!(f, "{}", err),
      RpcError::SerdeError(err) => write!(f, "{}", err),
      v => write!(f, "{:?}", v),
    }
  }
}

impl From<io::Error> for RpcError {
    fn from(err: io::Error) -> Self {
        RpcError::IoError(err)
    }
}

impl From<serde_json::error::Error> for RpcError {
    fn from(err: serde_json::error::Error) -> Self {
        RpcError::SerdeError(err)
    }
}

pub type RpcResult<T> = Result<T, RpcError>;
