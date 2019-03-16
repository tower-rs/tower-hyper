use futures::{future, Future, Stream};
use http::{Request, Uri};
use hyper::client::conn::Builder;
use hyper::client::connect::{Destination, HttpConnector};
use hyper::rt;
use tower::MakeService;
use tower_buffer::Buffer;
use tower_hyper::client::Connect;
use tower_hyper::retry::{Body, RetryPolicy};
use tower_hyper::util::Connector;
use tower_retry::Retry;
use tower_service::Service;

fn main() {
    pretty_env_logger::init();
    rt::run(future::lazy(|| {
        let dst = Destination::try_from_uri(Uri::from_static("http://127.0.0.1:3000")).unwrap();
        let connector = Connector::new(HttpConnector::new(1));
        let mut hyper = Connect::new(connector, Builder::new());

        hyper
            .make_service(dst)
            .map_err(|err| eprintln!("Connect Error {:?}", err))
            .and_then(|conn| {
                let buf = Buffer::new(conn, 1).map_err(|_| panic!("Unable to spawn!"));

                let policy = RetryPolicy::new(5);

                let retry = Retry::new(policy, buf.unwrap());

                Buffer::new(retry, 1).map_err(|_| panic!("Unable to spawn!"))
            })
            .and_then(|mut conn| {
                conn.call(Request::new(Body::from(Vec::new())))
                    .map_err(|e| eprintln!("Call Error: {}", e))
                    .and_then(|response| {
                        println!("Response Status: {:?}", response.status());
                        response
                            .into_body()
                            .concat2()
                            .map_err(|e| eprintln!("Body Error: {}", e))
                    })
                    .and_then(|body| {
                        println!("Response Body: {:?}", body);
                        Ok(())
                    })
            })
    }));
}
