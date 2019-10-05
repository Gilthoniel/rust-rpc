use rpc::Server;
use rpc::group::Address;
use rpc::transport::tcp::{
    TcpClientTransport,
    TcpServerTransport,
};

#[test]
fn hello_world() {
    #[derive(Serialize, Deserialize, Debug)]
    pub enum HelloError {
        Error(String),
    };

    impl<E: std::error::Error + Sized> std::convert::From<E> for HelloError {
        fn from(err: E) -> Self {
            HelloError::Error(err.description().to_string())
        }
    }

    #[rpc_macro::service]
    trait Hello {
        fn hello(&self, ctx: Context, arg: String) -> Result<String, HelloError>;
    }

    let addr = Address::from_str("127.0.0.1:2000");

    struct HelloService;

    impl Hello for HelloService {
        fn hello(&self, _: Context, arg: String) -> Result<String, HelloError> {
            Ok(arg)
        }
    }

    let mut srv = Server::new();
    let service = HelloService {};
    srv.run(
        service.get_processor(),
        TcpServerTransport::new(addr.clone()).unwrap(),
    );

    let c = HelloClient::new(TcpClientTransport::new(addr));
    let msg = String::from("deadbeef");
    let r = c.hello(msg.clone()).unwrap();
    assert_eq!(msg, r);
}
