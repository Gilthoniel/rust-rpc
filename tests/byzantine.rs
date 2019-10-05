use rpc::Server;
use rpc::group::Address;
use rpc::transport::tcp::{
    TcpClientTransport,
    TcpServerTransport,
};

#[test]
fn error() -> Result<(), ()> {
    #[derive(Serialize, Deserialize, Debug)]
    pub enum ByzantineError {
        Error(String),
    }

    impl<E: std::error::Error + Sized> std::convert::From<E> for ByzantineError {
        fn from(err: E) -> Self {
            ByzantineError::Error(err.description().to_string())
        }
    }

    #[rpc_macro::service]
    trait Byzantine {
        fn byzantine(&self, ctx: Context, arg: u64) -> Result<u64, ByzantineError>;
    }

    struct ByzantineService;

    impl Byzantine for ByzantineService {
        fn byzantine(&self, _: Context, _: u64) -> Result<u64, ByzantineError> {
            panic!("example panic in test");
        }
    }

    let addr = Address::from_str("127.0.0.1:2002");

    let mut srv = Server::new();
    let service = ByzantineService;

    srv.run(
        service.get_processor(),
        TcpServerTransport::new(addr.clone()).unwrap(),
    );

    let c = ByzantineClient::new(TcpClientTransport::new(addr));
    assert!(c.byzantine(0).is_err());

    Ok(())
}
