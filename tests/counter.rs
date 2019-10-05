use std::sync::atomic::{AtomicU64, Ordering};
use rpc::Server;
use rpc::group::Address;
use rpc::transport::tcp::{
    TcpClientTransport,
    TcpServerTransport,
};

#[test]
#[should_panic(expected = "IncrementError")]
fn counter() {
    #[derive(Serialize, Deserialize, Debug)]
    pub enum CounterError {
        IncrementError,
        Error(String),
    };

    impl<E: std::error::Error + Sized> std::convert::From<E> for CounterError {
        fn from(err: E) -> Self {
            CounterError::Error(err.description().to_string())
        }
    }

    #[rpc_macro::service]
    trait Counter {
        fn counter(&self, ctx: Context, v: u64) -> Result<u64, CounterError>;
        fn fetch(&self, ctx: Context, v: u64) -> Result<u64, CounterError>;
    }

    struct CounterService {
        value: AtomicU64,
    }

    impl Counter for CounterService {
        fn counter(&self, _: Context, v: u64) -> Result<u64, CounterError> {
            if v == 0 {
                return Err(CounterError::IncrementError);
            }

            let prev = self.value.fetch_add(v, Ordering::Relaxed);
            Ok(prev + v)
        }

        fn fetch(&self, _: Context, _: u64) -> Result<u64, CounterError> {
            Ok(self.value.load(Ordering::Relaxed))
        }
    }

    let addr = Address::from_str("127.0.0.1:2001");

    let mut srv = Server::new();
    let service = CounterService {
        value: AtomicU64::new(0),
    };
    srv.run(
        service.get_processor(),
        TcpServerTransport::new(addr.clone()).unwrap(),
    );

    let mut threads = Vec::new();
    let n = 5;
    let k = 10;
    for _ in 0..n {
        let addr = addr.clone();
        let h = std::thread::spawn(move || {
            let c = CounterClient::new(TcpClientTransport::new(addr));

            for _ in 0..k {
                c.counter(1).unwrap();
            }
        });

        threads.push(h);
    }

    for th in threads {
        th.join().unwrap();
    }

    let c = CounterClient::new(TcpClientTransport::new(addr));
    let r = c.fetch(0).unwrap();
    assert_eq!(r, n * k);

    // This is where it should panic.
    c.counter(0).unwrap();
}
